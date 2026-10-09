"""Copy the campaign route from the poe2speedrun repo into ui/guide.json.

Usage: uv run scripts/import_guide.py [path/to/poe2]   (default: ../poe2)
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT.parent / "poe2"


def i18n(value):
    if value is None:
        return None
    if isinstance(value, dict) and "text" in value:
        value = value["text"]
    if isinstance(value, str):
        return {"de": value, "en": value}
    return {"de": value.get("de", value.get("en", "")), "en": value.get("en", value.get("de", ""))}


def current(obj, league):
    """Drop entries tagged for an older patch (same rule as build.py)."""
    lg = obj.get("league") if isinstance(obj, dict) else None
    return not lg or league.startswith(lg)


def main():
    guide = json.loads((SRC / "guide.json").read_text(encoding="utf-8"))
    league = guide["meta"].get("league", "")
    acts, steps = [], []
    for path in guide["acts"]:
        act = json.loads((SRC / path).read_text(encoding="utf-8"))
        acts.append({"id": act["id"], "short": i18n(act["short"]), "title": i18n(act["title"])})
        for s in act["steps"]:
            if not current(s, league):
                continue
            steps.append({
                "act": len(acts) - 1,
                "zone": s.get("zone", ""),
                "tags": s.get("tags", []),
                "level": s.get("level"),
                "text": i18n(s["text"]),
                "hint": i18n(s.get("hint")),
                "notes": [i18n(n) for n in s.get("notes", []) if current(n, league)],
                "quest": i18n(s.get("quest")),
            })
    meta = {k: guide["meta"].get(k) for k in ("patch", "league", "updated")}
    meta["source"] = "https://poe2speedrun.malura.de/"
    out = ROOT / "ui" / "guide.json"
    out.write_text(json.dumps({"meta": meta, "acts": acts, "steps": steps}, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"{len(acts)} acts, {len(steps)} steps -> {out}")


if __name__ == "__main__":
    main()
