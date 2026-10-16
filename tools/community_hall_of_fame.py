#!/usr/bin/env python3
"""Publish evidence-based community ranks without touching application state."""
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import re
import subprocess
import tempfile
from urllib.parse import quote

RANKS = [
    {"minimum_xp": minimum, "title": title}
    for minimum, title in [
        (0, "Telegram Trainee"), (10, "Bus Bug Scout"),
        (25, "Group Address Gumshoe"), (50, "Topology Trouble Detective"),
        (100, "Commissioning Chaos Tamer"), (150, "Parameter Poltergeist Hunter"),
        (225, "Datapoint Dragon Slayer"), (325, "Tunnel Trouble Exorcist"),
        (450, "Routing Riddle Resolver"), (650, "Application Apocalypse Survivor"),
        (900, "Master of Missing Acknowledgements"), (1250, "Grandmaster of the Green Bus"),
        (1750, "Keeper of the Unbroken Bus"), (2500, "Lord of the Last Regression"),
        (4000, "Supreme Commander of the Blinking LED"), (6000, "Legend of the Twisted Pair"),
    ]
]
# Only earned names enter generated documents/data; the public goals are hints.
REPORT_MEDALS = [
    (10, "Golden Bus Probe"),
    (20, "Double-Decade Fault Finder"),
    (30, "Topology Treasure Hunter"),
    (40, "Four-Line Fault Patrol"),
    (50, "Half-Century Bus Sleuth"),
    (60, "Hexadecimal Hunch Master"),
    (70, "Seven-Area Signal Scout"),
    (80, "Octet of Observations"),
    (90, "Almost-Centurion Antenna"),
    (100, "Century of Bus Mysteries"),
]
RESOLUTION_MEDALS = [
    (5, "Loose End Lasso"),
    (10, "Regression Repair Ribbon"),
    (15, "Acknowledgement Ace"),
    (20, "Closed-Loop Champion"),
    (25, "Quarter-Century Quencher"),
    (30, "Topology Peacekeeper"),
    (35, "Phantom Fault Buster"),
    (40, "Bus Serenity Badge"),
    (45, "Tunnel Light Keeper"),
    (50, "Half-Century Fix Beacon"),
    (55, "Parameter Peace Prize"),
    (60, "Sixty Solutions Salute"),
    (65, "Datapoint Harmony Seal"),
    (70, "Routing Rescue Star"),
    (75, "Three-Quarter Victory Lap"),
    (80, "Octet of Quiet Lines"),
    (85, "Lasting Fix Laureate"),
    (90, "Regression Retirement Medal"),
    (95, "Nearly Mythical Maintainer Ally"),
    (100, "Centurion of the Unbroken Bus"),
]


PULL_REQUEST_MEDALS = [
    (1, "Patch Pilot"), (5, "Patch Panel Pro"), (10, "Pull Request Pathfinder"),
    (20, "Two-Decade Code Courier"), (50, "Half-Century Patch Patrol"),
    (100, "Centurion of Contributions"),
]
MERGE_MEDALS = [
    (1, "Green Merge Ribbon"), (5, "Five-Star Fix Welder"), (10, "Mainline Mechanic"),
    (20, "Merge Junction Master"), (50, "Half-Century Mainline Medal"),
    (100, "Centurion of the Green Mainline"),
]


def medals_for(issues, resolutions, pull_requests=0, merges=0):
    return ([name for minimum, name in REPORT_MEDALS if issues >= minimum] +
            [name for minimum, name in RESOLUTION_MEDALS if resolutions >= minimum] +
            [name for minimum, name in PULL_REQUEST_MEDALS if pull_requests >= minimum] +
            [name for minimum, name in MERGE_MEDALS if merges >= minimum])


START = "<!-- HALL-OF-FAME:START -->"
END = "<!-- HALL-OF-FAME:END -->"
BOT = "github-actions[bot]"


def positive(value):
    if type(value) is not int or value <= 0:
        raise ValueError("expected a positive integer")
    return value


def login(value):
    if not isinstance(value, str) or not re.fullmatch(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})(?:\[bot\])?", value):
        raise ValueError("invalid public account identifier")
    return value


def timestamp(value):
    if not isinstance(value, str):
        raise ValueError("timestamp must be text")
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        raise ValueError("timestamp must include a timezone")
    return parsed


def rank_for(xp):
    return next(rank.copy() for rank in reversed(RANKS) if xp >= rank["minimum_xp"])


def validate_policy(policy):
    for key in ["issue_xp", "reply_xp", "resolution_xp",
                "pull_request_xp", "pull_request_closed_xp", "pull_request_merged_xp"]:
        positive(policy[key])
    if policy["pull_request_merged_xp"] <= policy["pull_request_closed_xp"]:
        raise ValueError("merged PR bonus must exceed closed-unmerged bonus")
    timestamp(policy["notify_since"])
    if not isinstance(policy["excluded_logins"], list):
        raise ValueError("excluded_logins must be a list")
    trusted = policy.get("trusted_notice_logins", [])
    if not isinstance(trusted, list):
        raise ValueError("trusted_notice_logins must be a list")
    for value in trusted:
        login(value)
    return {BOT.casefold()} | {login(value).casefold() for value in policy["excluded_logins"]}


def participant(actor, excluded):
    if actor is None:
        return None
    if actor.get("type") not in {"User", "Organization", "Bot"}:
        raise ValueError("unknown GitHub actor type")
    value = login(actor["login"])
    return value if value.casefold() not in excluded else None


def unique(records, key):
    result = {}
    for record in records:
        identifier = positive(record[key])
        if identifier in result and result[identifier] != record:
            raise ValueError("conflicting duplicate API records; retry the snapshot")
        result[identifier] = record
    return list(result.values())


def validate_previous(previous, repository):
    if (type(previous.get("schema_version")) is not int or
            previous["schema_version"] not in {1, 2} or previous.get("repository") != repository):
        raise ValueError("unsupported or foreign community snapshot")
    awards = previous["resolution_awards"]
    if not isinstance(awards, dict):
        raise ValueError("resolution awards must be an object")
    for number, award in awards.items():
        if str(positive(int(number))) != number:
            raise ValueError("invalid award issue identifier")
        login(award["login"]); positive(award["xp"]); timestamp(award["closed_at"])
        if type(award["notify"]) is not bool or type(award["notified"]) is not bool:
            raise ValueError("invalid notification state")
    return json.loads(json.dumps(awards))


def build_snapshot(issues, comments, policy, repository, now, previous=None, *, pull_requests=()):
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("invalid repository identifier")
    timestamp(now)
    excluded = validate_policy(policy)
    awards = validate_previous(previous, repository) if previous else {}
    rows = {}

    def row(who):
        return rows.setdefault(who.casefold(), {"login": who, "issues": 0, "replies": 0,
                                               "resolutions": 0, "resolution_xp": 0,
                                               "pull_requests": 0, "pull_requests_open": 0,
                                               "pull_requests_closed": 0, "pull_requests_merged": 0})

    real_issues = {}
    for item in unique(issues, "id"):
        if "pull_request" in item:
            continue
        number = positive(item["number"])
        if number in real_issues:
            raise ValueError("duplicate issue number")
        if item["state"] not in {"open", "closed"}:
            raise ValueError("unknown issue state")
        real_issues[number] = item
        who = participant(item.get("user"), excluded)
        if who is None:
            continue
        row(who)["issues"] += 1
        if item["state"] == "closed" and item.get("state_reason") == "completed":
            closed_at = item["closed_at"]; timestamp(closed_at)
            awards.setdefault(str(number), {"login": who, "xp": policy["resolution_xp"],
                "closed_at": closed_at, "notify": timestamp(closed_at) >= timestamp(policy["notify_since"]),
                "notified": False})
    for comment in unique(comments, "id"):
        match = re.fullmatch(r"https://api\.github\.com/repos/" + re.escape(repository) + r"/issues/([1-9][0-9]*)", comment["issue_url"])
        if not match:
            raise ValueError("foreign or invalid comment issue URL")
        if int(match[1]) not in real_issues:
            continue  # Pull-request conversations are deliberately outside this scoreboard.
        who = participant(comment.get("user"), excluded)
        if who is not None:
            row(who)["replies"] += 1
    for award in awards.values():
        if award["login"].casefold() not in excluded:
            target = row(award["login"])
            target["resolutions"] += 1; target["resolution_xp"] += award["xp"]
    pull_numbers = set()
    for item in unique(pull_requests, "id"):
        number = positive(item["number"])
        if number in pull_numbers or number in real_issues:
            raise ValueError("duplicate or conflicting pull request number")
        pull_numbers.add(number)
        if item["state"] not in {"open", "closed"}:
            raise ValueError("unknown pull request state")
        merged_at = item["merged_at"]  # /issues cannot establish a PR's merge status.
        if merged_at is not None:
            timestamp(merged_at)
            if item["state"] != "closed":
                raise ValueError("merged pull request must be closed")
        if item["state"] == "closed":
            timestamp(item["closed_at"])
        elif item["closed_at"] is not None:
            raise ValueError("open pull request cannot have a closure time")
        who = participant(item.get("user"), excluded)
        if who is None:
            continue
        target = row(who)
        target["pull_requests"] += 1
        state = "merged" if merged_at is not None else item["state"]
        target[f"pull_requests_{state}"] += 1
    contributors = list(rows.values())
    for target in contributors:
        target["xp_breakdown"] = {
            "issues": target["issues"] * policy["issue_xp"],
            "replies": target["replies"] * policy["reply_xp"],
            "resolutions": target.pop("resolution_xp"),
            "pull_requests": (target["pull_requests"] * policy["pull_request_xp"] +
                              target["pull_requests_closed"] * policy["pull_request_closed_xp"] +
                              target["pull_requests_merged"] * policy["pull_request_merged_xp"]),
        }
        target["xp"] = sum(target["xp_breakdown"].values())
        target["rank"] = rank_for(target["xp"])
        target["medals"] = medals_for(target["issues"], target["resolutions"],
                                      target["pull_requests"], target["pull_requests_merged"])
    contributors.sort(key=lambda target: (-target["xp"], target["login"].casefold()))
    for index, target in enumerate(contributors):
        target["position"] = index + 1 if index == 0 or target["xp"] != contributors[index - 1]["xp"] else contributors[index - 1]["position"]
    snapshot = {"schema_version": 2, "repository": repository, "generated_at": now,
                "policy": policy.copy(), "ranks": RANKS, "resolution_awards": awards,
                "contributors": contributors}
    if previous and {k: v for k, v in snapshot.items() if k != "generated_at"} == {k: v for k, v in previous.items() if k != "generated_at"}:
        snapshot["generated_at"] = previous["generated_at"]
    return snapshot


def render_table(snapshot):
    lines = ["| Place | Bug hunter | Rank & title | Medal | XP | Issues | Replies | Resolved | PRs | Open PRs | Merged PRs | Closed unmerged | XP sources |",
             "| ---: | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |"]
    for target in snapshot["contributors"]:
        who = target["login"]
        medal = "<br>".join(f"🏅 {name}" for name in target["medals"]) or "—"
        xp = target["xp_breakdown"]
        sources = (f"{xp['issues']} issue + {xp['replies']} reply + "
                   f"{xp['resolutions']} completion + {xp['pull_requests']} PR")
        lines.append(f"| {target['position']} | [@{who}](https://github.com/{quote(who, safe='')}) | {target['rank']['title']} | {medal} | **{target['xp']}** | {target['issues']} | {target['replies']} | {target['resolutions']} | {target['pull_requests']} | {target['pull_requests_open']} | {target['pull_requests_merged']} | {target['pull_requests_closed']} | {sources} |")
    if not snapshot["contributors"]:
        lines.append("| — | Your first report belongs here. | Telegram Trainee | — | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | — |")
    return "\n".join(lines)


def replace_readme(text, snapshot):
    if text.count(START) != 1 or text.count(END) != 1 or text.index(START) >= text.index(END):
        raise ValueError("README must contain exactly one ordered hall-of-fame marker pair")
    before, rest = text.split(START); _, after = rest.split(END)
    content = "\n" + render_table(snapshot) + f"\n\n*Snapshot: {snapshot['generated_at']}. Maintainers and the scoreboard bot sit this one out.*\n"
    return before + START + content + END + after


def render_page(snapshot):
    policy = snapshot["policy"]
    title = "# Bug Hunters’ Hall of Fame\n\n"
    intro = "**Every report makes the bus a little less mysterious.** This is our public\nthank-you to reporters who test KNXBench, reproduce surprises and come back\nwith the details. You do not need to write code to earn a place here.\n\n"
    rules = f"""## How the points work

- **+{policy['issue_xp']} XP per issue** opened in this repository, open or closed.
- **+{policy['reply_xp']} XP per reply** on an issue, including follow-up-only contributors.
- **+{policy['resolution_xp']} XP for the reporter** when an issue is closed as **completed**.
  This bonus is awarded once per issue and retained if it is reopened. Closing
  as **not planned** does not earn a completion bonus.
- **+{policy['pull_request_xp']} XP per pull request** submitted, including open/draft PRs.
- **+{policy['pull_request_merged_xp']} XP merge bonus**: **{policy['pull_request_xp'] + policy['pull_request_merged_xp']} XP total** per merged PR.
- **+{policy['pull_request_closed_xp']} XP participation bonus**: **{policy['pull_request_xp'] + policy['pull_request_closed_xp']} XP total** per PR closed without merge.
  Closed unmerged does not necessarily mean rejected. Open, merged and closed
  unmerged are mutually exclusive states; the two bonuses never stack.
- **XP sources** show issue, reply, completion and PR points separately; their
  sum is the displayed XP total. PRs are not counted again as issues.

## The next mystery

- **Reporter milestones:** a new medal waits at every ten reports, up to 100.
  Twenty reports? There is more than one bus probe in the trophy cabinet.
- **Resolution milestones:** every five reports closed as **completed** unlock
  another medal, up to 100. Reopening and closing the same issue is not a shortcut.
- **PR milestones:** medals at 1, 5, 10, 20, 50 and 100 submitted PRs, plus a
  separate track at the same milestones for merged PRs. Closing without merge
  earns participation XP, but does not advance the merge-medal track.
- **No spoilers:** medal names appear only once earned, in the table and completion
  acknowledgements. Keep reporting and helping us close the loop to discover them.
  These are thank-you medals, not extra XP — and no hardware needs shelf space.

Reporters, issue-thread repliers and PR authors are listed, not just the top ten. External
AI agents are welcome to earn points too. Maintainer accounts, explicit opt-outs
and the scoreboard bot itself are excluded. PR discussion/review comments, reviews, reactions,
Discussions and private messages are not counted. Tied scores share a place;
alphabetical order merely keeps the table stable. Counts measure participation,
not report quality, correctness, or a promise that every request will be implemented.

Reply edits do not create points. Deleted replies stop counting at the next
refresh; retained completion awards are not silently discarded when an issue
is deleted or transferred. Historical completed issues receive their bonus
without posting historical award comments. New completed issues receive an
automatic acknowledgement with earned XP, total XP and rank/title (and a
rank-up announcement if the bonus crosses a threshold). Retry/reopen does not
post a second award notice for the same issue.

PR counters, XP and medals follow the current API state. Reopening a closed
unmerged PR returns it to submission XP; merging it replaces, rather than stacks,
the closed-unmerged bonus. A removed PR stops counting. Historical PRs count
immediately, without posting PR award comments or changing issue notices.

This page and the README refresh on issue/reply and PR lifecycle events and daily, with manual
refresh available. GitHub API or permission failures fail the job rather than
publish invented counts. Schedule delays or outages can leave the displayed
snapshot behind; the timestamp states what was actually generated.

Only public GitHub handles, counters, award issue numbers and dates are stored;
no report bodies, names, emails or project files enter this data. Want your
handle removed? Ask a maintainer; the explicit exclusion list is in
`docs/community/hall-of-fame-policy.json`.

"""
    ranking = "## Ranks and suitably over-engineered titles\n\n| XP from | Title |\n| ---: | --- |\n"
    ranking += "\n".join(f"| {rank['minimum_xp']} | {rank['title']} |" for rank in RANKS)
    return (title + intro + "## The field-test crew\n\n" + render_table(snapshot) +
            f"\n\n*Snapshot: {snapshot['generated_at']}.*\n\n" + rules + ranking +
            "\n\n**Found a bug? You found an opportunity for promotion.**\n").rstrip() + "\n"


def closing_note(snapshot, number):
    award = snapshot["resolution_awards"][str(number)]
    target = next(row for row in snapshot["contributors"] if row["login"].casefold() == award["login"].casefold())
    previous_rank = rank_for(target["xp"] - award["xp"])["title"]
    current_rank = target["rank"]["title"]
    rank_up = f"\n\n**Rank up:** {previous_rank} → **{current_rank}**." if current_rank != previous_rank else ""
    medal = ("\n\n**Medals earned:** " + " · ".join(f"🏅 {name}" for name in target["medals"])) if target["medals"] else ""
    return (f"🏅 **Field-test credit for @{target['login']}: +{award['xp']} XP** for this completed issue.\n\n"
            f"**Total: {target['xp']} XP · Title: {current_rank}**\n"
            f"{target['issues']} issues · {target['replies']} replies · {target['resolutions']} completed reports."
            + rank_up + medal + "\n\nThanks for making KNXBench harder to break. The bus salutes you.\n\n"
            f"[Bug Hunters’ Hall of Fame](https://github.com/{snapshot['repository']}/blob/HEAD/docs/community/HALL_OF_FAME.md)\n\n"
            f"<!-- knxbench-field-credit:{number} -->")


def notify_pending(snapshot, api):
    excluded = validate_policy(snapshot["policy"])
    trusted_authors = {value.casefold() for value in snapshot["policy"].get("trusted_notice_logins", [])}
    for identifier, award in sorted(snapshot["resolution_awards"].items(), key=lambda pair: int(pair[0])):
        if not award["notify"] or award["notified"] or award["login"].casefold() in excluded:
            continue
        number = int(identifier); live = api.get_issue(number)
        if live["state"] != "closed" or live.get("state_reason") != "completed":
            continue
        marker = f"<!-- knxbench-field-credit:{number} -->"
        trusted = [comment for comment in api.get_comments(number)
                   if comment.get("user") and
                   (comment["user"].get("login", "").casefold() in trusted_authors or
                    (comment["user"].get("login") == BOT and comment["user"].get("type") == "Bot"))]
        if not any(marker in comment.get("body", "") for comment in trusted):
            body = closing_note(snapshot, number)
            posted = api.post_comment(number, body)
            receipt = api.get_comment(positive(posted["id"]))
            if receipt.get("id") != posted["id"] or receipt.get("body") != body:
                raise ValueError("award comment did not pass exact readback")
        award["notified"] = True


class GitHub:
    def __init__(self, repository):
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
            raise ValueError("invalid repository identifier")
        self.repository = repository

    def api(self, endpoint, *, pages=False, body=None):
        command = ["gh", "api", endpoint]
        if pages:
            command += ["--paginate", "--slurp"]
        if body is not None:
            command += ["--method", "POST", "--input", "-"]
        result = subprocess.run(command, input=json.dumps({"body": body}) if body is not None else None,
                                text=True, capture_output=True, check=True, timeout=180)
        payload = json.loads(result.stdout)
        if pages:
            if not isinstance(payload, list) or not all(isinstance(page, list) for page in payload):
                raise ValueError("invalid paginated GitHub response")
            return [item for page in payload for item in page]
        return payload

    def get_issue(self, number):
        return self.api(f"repos/{self.repository}/issues/{positive(number)}")

    def get_comments(self, number):
        return self.api(f"repos/{self.repository}/issues/{positive(number)}/comments?per_page=100", pages=True)

    def post_comment(self, number, body):
        return self.api(f"repos/{self.repository}/issues/{positive(number)}/comments", body=body)

    def get_comment(self, cid):
        return self.api(f"repos/{self.repository}/issues/comments/{positive(cid)}")


def write_outputs(root, snapshot):
    readme = root / "README.md"
    targets = {
        readme: replace_readme(readme.read_text(), snapshot),
        root / "docs/community/HALL_OF_FAME.md": render_page(snapshot),
        root / "docs/community/hall-of-fame.json": json.dumps(snapshot, ensure_ascii=False, indent=2) + "\n",
    }
    # Render and validate every payload before replacing any existing file.
    for path, text in targets.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=path.parent, delete=False) as stream:
            stream.write(text); temp = Path(stream.name)
        try:
            temp.replace(path)
        finally:
            temp.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--repository", default="KNXBench-Labs/KNXBench")
    parser.add_argument("--notify", action="store_true", help="publish pending completion-credit comments")
    args = parser.parse_args()
    policy = json.loads((args.root / "docs/community/hall-of-fame-policy.json").read_text())
    old_file = args.root / "docs/community/hall-of-fame.json"
    previous = json.loads(old_file.read_text()) if old_file.exists() else None
    # Validation happens before network access or externally visible writes.
    validate_policy(policy)
    api = GitHub(args.repository)
    issues = api.api(f"repos/{args.repository}/issues?state=all&per_page=100", pages=True)
    comments = api.api(f"repos/{args.repository}/issues/comments?per_page=100", pages=True)
    pulls = api.api(f"repos/{args.repository}/pulls?state=all&per_page=100", pages=True)
    now = datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")
    snapshot = build_snapshot(issues, comments, policy, args.repository, now, previous,
                              pull_requests=pulls)
    replace_readme((args.root / "README.md").read_text(), snapshot)
    if args.notify:
        notify_pending(snapshot, api)
    write_outputs(args.root, snapshot)
    print(json.dumps({"contributors": len(snapshot["contributors"]), "generated_at": snapshot["generated_at"],
                      "issues": sum(row["issues"] for row in snapshot["contributors"]),
                      "replies": sum(row["replies"] for row in snapshot["contributors"]),
                      "pull_requests": sum(row["pull_requests"] for row in snapshot["contributors"]),
                      "pull_requests_merged": sum(row["pull_requests_merged"] for row in snapshot["contributors"]),
                      "pull_requests_closed": sum(row["pull_requests_closed"] for row in snapshot["contributors"])}, sort_keys=True))


if __name__ == "__main__":
    main()
