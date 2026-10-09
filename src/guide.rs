//! Moves the guide pointer when the player enters a zone.

use serde::Deserialize;

/// How far ahead a zone change may jump. Keeps a waypoint trip through an
/// old zone from skipping half an act.
const LOOKAHEAD: usize = 8;

#[derive(Deserialize)]
struct RawStep {
    zone: String,
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct RawGuide {
    steps: Vec<RawStep>,
}

pub struct Guide {
    /// Per step: normalized zone names ("A -> B" counts as both) and whether it is a town step.
    steps: Vec<(Vec<String>, bool)>,
}

fn norm(zone: &str) -> String {
    let z = zone.trim().to_lowercase();
    z.strip_prefix("the ").unwrap_or(&z).to_string()
}

impl Guide {
    pub fn parse(json: &str) -> serde_json::Result<Self> {
        let raw: RawGuide = serde_json::from_str(json)?;
        let steps = raw
            .steps
            .into_iter()
            .map(|s| {
                let zones = s.zone.split("->").flat_map(|p| p.split('/')).map(norm).filter(|z| !z.is_empty()).collect();
                (zones, s.tags.iter().any(|t| t == "town"))
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
        let matches = |i: usize| self.steps[i].0.contains(&z);
        if cur >= self.steps.len() || matches(cur) {
            return None;
        }
        if cur > 0 && self.steps[cur].1 && matches(cur - 1) {
            return Some(cur - 1);
        }
        (cur + 1..self.steps.len().min(cur + 1 + LOOKAHEAD)).find(|&i| matches(i) && (!self.steps[i].1 || i == cur + 1))
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
}
