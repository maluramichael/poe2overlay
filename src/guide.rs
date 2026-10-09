//! Moves the guide pointer when the player enters a zone or gets a step's reward.

use serde::Deserialize;

/// How far ahead a zone change may jump. Keeps a waypoint trip through an
/// old zone from skipping half an act.
const LOOKAHEAD: usize = 8;
/// How far ahead a reward may complete a step. Passive points come from many
/// steps, so this stays short.
const REWARD_LOOKAHEAD: usize = 4;

/// Permanent rewards: phrase in the step's English reward (after "->") and the
/// text PoE2 writes to Client.txt when you get it. Log texts taken from real logs:
/// ": gem_override has received +10% to [Resistances|Cold Resistance]."
/// ": You have received 2 Passive Skill Points."
const REWARDS: &[(&str, &str)] = &[
    ("cold resistance", "Cold Resistance]"),
    ("fire resistance", "Fire Resistance]"),
    ("lightning resistance", "Lightning Resistance]"),
    ("+30 spirit", "+30 to [Spirit|Spirit]"),
    ("+40 spirit", "+40 to [Spirit|Spirit]"),
    ("passive points", "Passive Skill Points"),
    ("max. mana", "increased maximum Mana"),
    ("max. life", "increased maximum Life"),
    ("+20 life", "+20 to maximum Life"),
    ("charm slot", "[Charm] Slot"),
];

#[derive(Deserialize, Default)]
struct Text {
    en: String,
}

#[derive(Deserialize)]
struct RawStep {
    zone: String,
    tags: Vec<String>,
    #[serde(default)]
    text: Text,
}

#[derive(Deserialize)]
struct RawGuide {
    steps: Vec<RawStep>,
}

struct Step {
    /// Normalized zone names ("A -> B" counts as both).
    zones: Vec<String>,
    town: bool,
    /// Log texts that mean this step is done.
    rewards: Vec<&'static str>,
}

pub struct Guide {
    steps: Vec<Step>,
}

fn norm(zone: &str) -> String {
    let z = zone.trim().to_lowercase();
    z.strip_prefix("the ").unwrap_or(&z).to_string()
}

fn rewards(en: &str) -> Vec<&'static str> {
    let Some((_, reward)) = en.split_once("->") else { return vec![] };
    let reward = reward.to_lowercase();
    REWARDS.iter().filter(|(phrase, _)| reward.contains(phrase)).map(|(_, log)| *log).collect()
}

impl Guide {
    pub fn parse(json: &str) -> serde_json::Result<Self> {
        let raw: RawGuide = serde_json::from_str(json)?;
        let steps = raw
            .steps
            .into_iter()
            .map(|s| Step {
                zones: s.zone.split("->").flat_map(|p| p.split('/')).map(norm).filter(|z| !z.is_empty()).collect(),
                town: s.tags.iter().any(|t| t == "town"),
                rewards: rewards(&s.text.en),
            })
            .collect();
        Ok(Guide { steps })
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// New pointer after entering `zone`, or None to stay.
    /// Town steps further than one step ahead are skipped: a portal back to
    /// sell must not jump over the rest of the current zone's steps. And when
    /// that portal did land on the next town step, walking back through it
    /// into the previous zone undoes the jump.
    pub fn advance(&self, cur: usize, zone: &str) -> Option<usize> {
        let z = norm(zone);
        let matches = |i: usize| self.steps[i].zones.contains(&z);
        if cur >= self.steps.len() || matches(cur) {
            return None;
        }
        if cur > 0 && self.steps[cur].town && matches(cur - 1) {
            return Some(cur - 1);
        }
        (cur + 1..self.steps.len().min(cur + 1 + LOOKAHEAD)).find(|&i| matches(i) && (!self.steps[i].town || i == cur + 1))
    }

    /// New pointer after a reward line from the log: the step after the one
    /// that gives this reward, or None to stay.
    pub fn complete(&self, cur: usize, reward: &str) -> Option<usize> {
        (cur..self.steps.len().min(cur + REWARD_LOOKAHEAD))
            .find(|&i| self.steps[i].rewards.iter().any(|r| reward.contains(r)))
            .map(|i| (i + 1).min(self.steps.len() - 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guide() -> Guide {
        Guide::parse(
            r#"{"steps":[
            {"zone":"The Riverbank","tags":["must"]},
            {"zone":"Clearfell Encampment","tags":["town"]},
            {"zone":"Clearfell","tags":[]},
            {"zone":"The Grelwood","tags":[]},
            {"zone":"The Red Vale","tags":[]},
            {"zone":"Clearfell Encampment","tags":["town"]},
            {"zone":"Ngakanu -> Heart of the Tribe","tags":[]}
        ]}"#,
        )
        .unwrap()
    }

    #[test]
    fn follows_zone_changes() {
        let g = guide();
        assert_eq!(g.advance(0, "The Riverbank"), None);
        assert_eq!(g.advance(0, "Clearfell Encampment"), Some(1));
        assert_eq!(g.advance(1, "Clearfell"), Some(2));
        assert_eq!(g.advance(2, "Grelwood"), Some(3), "leading 'The' is ignored");
        assert_eq!(g.advance(3, "Hideout"), None);
    }

    #[test]
    fn town_portal_does_not_skip_ahead() {
        let g = guide();
        assert_eq!(g.advance(3, "Clearfell Encampment"), None);
        assert_eq!(g.advance(4, "Clearfell Encampment"), Some(5));
        assert_eq!(g.advance(5, "The Red Vale"), Some(4), "portal back into the zone");
    }

    #[test]
    fn never_moves_back_and_splits_combined_zones() {
        let g = guide();
        assert_eq!(g.advance(4, "The Riverbank"), None);
        assert_eq!(g.advance(5, "Heart of the Tribe"), Some(6));
    }

    #[test]
    fn bundled_guide_parses() {
        let g = Guide::parse(include_str!("../ui/guide.json")).unwrap();
        assert!(g.len() > 100);
        assert_eq!(g.advance(0, "Clearfell Encampment"), Some(1));
    }

    #[test]
    fn rewards_complete_their_step() {
        let g = Guide::parse(include_str!("../ui/guide.json")).unwrap();
        let cold = "+10% to [Resistances|Cold Resistance].";
        assert_eq!(g.complete(2, cold), Some(3), "Beira in Clearfell");
        assert_eq!(g.complete(0, cold), Some(3), "a few steps ahead is fine");
        assert_eq!(g.complete(3, cold), None);
        assert_eq!(g.complete(11, "+30 to [Spirit|Spirit]."), Some(12), "King in the Mists");
        assert_eq!(g.complete(10, "2 Passive Skill Points."), Some(11), "Crowbell");
        assert_eq!(g.complete(4, "2 Passive Skill Points."), None, "Crowbell is too far ahead");
    }
}
