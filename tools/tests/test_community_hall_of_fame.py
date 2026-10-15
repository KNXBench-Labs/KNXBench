"""Test real counting, retained awards and public recognition rendering."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "hall", Path(__file__).resolve().parents[1] / "community_hall_of_fame.py"
)
hall = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(hall)

NOW = "2026-10-13T20:00:00Z"
POLICY = {
    "excluded_logins": ["KNXBench-Labs", "Andre1Becker"],
    "issue_xp": 5, "reply_xp": 1, "resolution_xp": 10,
    "notify_since": NOW,
}


def user(login="tester", kind="User"):
    return {"login": login, "type": kind}


def issue(number=1, login="tester", state="open", reason=None, **extra):
    return {"id": number + 100, "number": number, "user": user(login),
            "state": state, "state_reason": reason,
            "closed_at": "2026-10-13T20:01:00Z" if state == "closed" else None,
            **extra}


def reply(cid=1, login="tester", number=1, kind="User", body="A useful detail"):
    return {"id": cid, "user": user(login, kind), "body": body,
            "issue_url": f"https://api.github.com/repos/example/project/issues/{number}"}


def build(issues, comments=(), previous=None):
    return hall.build_snapshot(issues, list(comments), POLICY, "example/project", NOW, previous)


class CountingTests(unittest.TestCase):
    def test_issue_and_reply_counters_are_separate(self):
        row = build([issue()], [reply()])["contributors"][0]
        self.assertEqual((row["issues"], row["replies"], row["xp"]), (1, 1, 6))

    def test_reply_only_contributor_is_listed(self):
        rows = build([issue()], [reply(login="helper")])["contributors"]
        helper = next(row for row in rows if row["login"] == "helper")
        self.assertEqual((helper["issues"], helper["replies"], helper["xp"]), (0, 1, 1))

    def test_maintainers_are_excluded_case_insensitively(self):
        rows = build([issue(login="knxbench-labs"), issue(2)],
                     [reply(login="Andre1Becker"), reply(2, login="helper")])["contributors"]
        self.assertEqual({row["login"] for row in rows}, {"tester", "helper"})

    def test_external_agents_count_but_the_scoreboard_bot_and_prs_do_not(self):
        pr = issue(2, "pr-author", pull_request={"url": "example"})
        bot = issue(3, "robot"); bot["user"]["type"] = "Bot"
        app = issue(4, "external-agent[bot]"); app["user"]["type"] = "Bot"
        own = issue(5, hall.BOT); own["user"]["type"] = "Bot"
        rows = build([issue(), pr, bot, app, own],
                     [reply(number=2), reply(2, login="robot", kind="Bot"),
                      reply(3, login=hall.BOT, kind="Bot"),
                      reply(4, login="external-agent[bot]", kind="Bot")])["contributors"]
        self.assertEqual({row["login"]: row["xp"] for row in rows},
                         {"tester": 5, "robot": 6, "external-agent[bot]": 6})
        self.assertIn("https://github.com/external-agent%5Bbot%5D", hall.render_table({"contributors": rows}))

    def test_external_bot_can_receive_completion_credit_and_medals(self):
        issues = [issue(i + 1, "external-agent[bot]", state="closed", reason="completed") for i in range(5)]
        for item in issues:
            item["user"]["type"] = "Bot"
        snapshot = build(issues)
        row = snapshot["contributors"][0]
        self.assertEqual((row["xp"], row["resolutions"], len(row["medals"])), (75, 5, 1))
        api = FakeApi(); hall.notify_pending(snapshot, api)
        self.assertEqual(len(api.posts), 5)
        self.assertIn("@external-agent[bot]", api.posts[0][1])
        self.assertEqual(build(issues, previous=snapshot)["contributors"], snapshot["contributors"])

    def test_own_bot_is_excluded_case_insensitively_and_external_bots_can_opt_out(self):
        policy = {**POLICY, "excluded_logins": POLICY["excluded_logins"] + ["external-agent[bot]"]}
        rows = hall.build_snapshot([issue(login="GitHub-Actions[bot]"), issue(2, "external-agent[bot]")],
                                   [], policy, "example/project", NOW)["contributors"]
        self.assertEqual(rows, [])

    def test_unknown_actor_types_still_fail_closed(self):
        item = issue(); item["user"]["type"] = "Unexpected"
        with self.assertRaises(ValueError):
            build([item])

    def test_workflow_admits_external_bot_replies_without_self_triggering(self):
        workflow = (Path(__file__).resolve().parents[2] / ".github/workflows/community-hall-of-fame.yml").read_text()
        self.assertIn("github.event.comment.user.login != 'github-actions[bot]'", workflow)
        self.assertNotIn("github.event.comment.user.type != 'Bot'", workflow)

    def test_duplicate_api_records_are_counted_once(self):
        row = build([issue(), issue()], [reply(), reply()])["contributors"][0]
        self.assertEqual(row["xp"], 6)

    def test_conflicting_duplicate_records_are_refused(self):
        with self.assertRaises(ValueError):
            build([issue(), issue(login="other")])

    def test_edited_reply_remains_one_reply(self):
        row = build([issue()], [reply(body="Edited detail")])["contributors"][0]
        self.assertEqual(row["replies"], 1)

    def test_completion_awarded_once_and_retained_on_reopen(self):
        closed = build([issue(state="closed", reason="completed")])
        self.assertEqual(closed["contributors"][0]["xp"], 15)
        reopened = build([issue()], previous=closed)
        self.assertEqual(reopened["contributors"][0]["xp"], 15)
        again = build([issue(state="closed", reason="completed")], previous=reopened)
        self.assertEqual(again["contributors"][0]["resolutions"], 1)
        self.assertEqual(again["contributors"][0]["xp"], 15)

    def test_not_planned_close_gets_no_resolution_award(self):
        row = build([issue(state="closed", reason="not_planned")])["contributors"][0]
        self.assertEqual((row["resolutions"], row["xp"]), (0, 5))

    def test_historical_resolutions_get_credit_without_spam(self):
        snapshot = build([issue(state="closed", reason="completed", closed_at="2026-01-01T00:00:00Z")])
        self.assertEqual(snapshot["contributors"][0]["xp"], 15)
        self.assertFalse(snapshot["resolution_awards"]["1"]["notify"])

    def test_many_contributors_are_not_truncated(self):
        rows = build([issue(i + 1, f"tester{i}") for i in range(250)])["contributors"]
        self.assertEqual(len(rows), 250)

    def test_sort_is_deterministic_with_shared_positions(self):
        rows = build([issue(1, "Zulu"), issue(2, "Alpha")])["contributors"]
        self.assertEqual([row["login"] for row in rows], ["Alpha", "Zulu"])
        self.assertEqual([row["position"] for row in rows], [1, 1])

    def test_existing_award_is_not_silently_deleted(self):
        previous = build([issue(state="closed", reason="completed")])
        empty = build([], previous=previous)
        self.assertEqual(empty["resolution_awards"], previous["resolution_awards"])
        self.assertEqual(empty["contributors"][0]["resolutions"], 1)

    def test_malformed_login_is_refused(self):
        with self.assertRaises(ValueError):
            build([issue(login="bad|[login]")])

    def test_unknown_state_is_refused(self):
        with self.assertRaises(ValueError):
            build([issue(state="surprise")])

    def test_unknown_previous_schema_is_refused(self):
        with self.assertRaises(ValueError):
            build([], previous={"schema_version": 999})

    def test_unchanged_input_keeps_snapshot_timestamp(self):
        previous = build([issue()])
        current = hall.build_snapshot([issue()], [], POLICY, "example/project",
                                      "2026-10-14T00:00:00Z", previous)
        self.assertEqual(current, previous)


class PresentationTests(unittest.TestCase):
    def test_rank_thresholds_include_many_english_titles(self):
        self.assertGreaterEqual(len(hall.RANKS), 15)
        for index, rank in enumerate(hall.RANKS):
            self.assertEqual(hall.rank_for(rank["minimum_xp"])["title"], rank["title"])
            if index:
                self.assertEqual(hall.rank_for(rank["minimum_xp"] - 1)["title"], hall.RANKS[index - 1]["title"])

    def test_report_medals_unlock_every_ten_reports_through_one_hundred(self):
        previous = []
        for count in range(101):
            with self.subTest(count=count):
                rows = build([issue(i + 1) for i in range(count)])["contributors"]
                medals = rows[0]["medals"] if rows else []
                self.assertEqual(len(medals), count // 10)
                self.assertEqual(len(medals), len(set(medals)))
                if count % 10:
                    self.assertEqual(medals, previous)
                previous = medals
        self.assertEqual(build([issue(i + 1) for i in range(105)])["contributors"][0]["medals"], previous)

    def test_completed_report_medals_unlock_every_five_through_one_hundred(self):
        previous = []
        for count in range(101):
            with self.subTest(count=count):
                issues = [issue(i + 1, state="closed", reason="completed") for i in range(count)]
                snapshot = build(issues)
                medals = snapshot["contributors"][0]["medals"] if count else []
                completion_medals = medals[count // 10:]
                self.assertEqual(len(completion_medals), count // 5)
                self.assertEqual(len(medals), len(set(medals)))
                if count % 5:
                    self.assertEqual(completion_medals, previous)
                previous = completion_medals
        beyond = build([issue(i + 1, state="closed", reason="completed") for i in range(105)])
        self.assertEqual(beyond["contributors"][0]["medals"][10:], previous)

    def test_medals_use_retained_completions_not_all_closed_issues(self):
        completed = [issue(i + 1, state="closed", reason="completed") for i in range(5)]
        initial = build(completed)
        self.assertEqual(len(initial["contributors"][0]["medals"]), 1)
        reopened = build([issue(i + 1) for i in range(5)], previous=initial)
        self.assertEqual(reopened["contributors"][0]["medals"], initial["contributors"][0]["medals"])
        deleted = build([], previous=initial)
        self.assertEqual(deleted["contributors"][0]["medals"], initial["contributors"][0]["medals"])
        uncompleted = build([issue(i + 1, state="closed", reason="not_planned") for i in range(5)])
        self.assertEqual(uncompleted["contributors"][0]["medals"], [])

    def test_ten_reports_award_the_golden_bus_probe(self):
        row = build([issue(i + 1) for i in range(10)])["contributors"][0]
        self.assertIn("Golden Bus Probe", row["medals"])

    def test_table_uses_requested_column_order_and_all_earned_medals(self):
        snapshot = build([issue(i + 1, state="closed", reason="completed") for i in range(20)])
        table = hall.render_table(snapshot)
        self.assertEqual(table.splitlines()[0],
                         "| Place | Bug hunter | Rank & title | Medal | XP | Issues | Replies | Resolved |")
        for medal in snapshot["contributors"][0]["medals"]:
            self.assertIn(medal, table)
            self.assertIn(medal, hall.closing_note(snapshot, 20))
        cells = table.splitlines()[2].split("|")[1:-1]
        self.assertEqual([cell.strip() for cell in cells[4:]], ["**300**", "20", "0", "20"])

    def test_empty_table_uses_the_same_column_order(self):
        cells = hall.render_table(build([])).splitlines()[2].split("|")[1:-1]
        self.assertEqual([cell.strip() for cell in cells],
                         ["—", "Your first report belongs here.", "Telegram Trainee", "—", "0", "0", "0", "0"])

    def test_public_goals_hint_without_revealing_unearned_medal_names(self):
        snapshot = build([issue(i + 1, state="closed", reason="completed") for i in range(20)])
        outputs = [hall.render_page(snapshot), hall.replace_readme(hall.START + hall.END, snapshot),
                   json.dumps(snapshot), hall.closing_note(snapshot, 20)]
        earned = snapshot["contributors"][0]["medals"]
        for output in outputs:
            for _, name in hall.REPORT_MEDALS + hall.RESOLUTION_MEDALS:
                self.assertEqual(name in output, name in earned, name)
        page = outputs[0]
        self.assertIn("## The next mystery", page)
        self.assertIn("every five", page)
        self.assertIn("100", page)
        self.assertNotIn("external human", page.lower())
        self.assertIn("Reporter", page)
        self.assertNotIn("Maintainers and bots", outputs[1])

    def test_every_contributor_is_in_the_public_table(self):
        snapshot = build([issue()], [reply(login="helper")])
        table = hall.render_table(snapshot)
        self.assertIn("@tester", table); self.assertIn("@helper", table)
        self.assertIn("Issues", table); self.assertIn("Replies", table)
        self.assertNotIn("KNXBench-Labs", table)

    def test_readme_replaces_only_owned_markers(self):
        text = "Before\n<!-- HALL-OF-FAME:START -->\nold\n<!-- HALL-OF-FAME:END -->\nAfter\n"
        result = hall.replace_readme(text, build([issue()]))
        self.assertTrue(result.startswith("Before\n")); self.assertTrue(result.endswith("After\n"))
        self.assertNotIn("\nold\n", result)

    def test_missing_or_duplicate_markers_are_refused(self):
        with self.assertRaises(ValueError):
            hall.replace_readme("No markers", build([issue()]))

    def test_close_note_reports_points_and_english_title(self):
        snapshot = build([issue(state="closed", reason="completed")])
        note = hall.closing_note(snapshot, 1)
        self.assertIn("+10 XP", note); self.assertIn("15 XP", note)
        self.assertIn("Bus Bug Scout", note); self.assertIn("Rank up", note)
        self.assertIn("<!-- knxbench-field-credit:1 -->", note)

    def test_close_without_rank_up_does_not_invent_one(self):
        snapshot = build([issue(i + 1) for i in range(10)] + [issue(11, state="closed", reason="completed")])
        note = hall.closing_note(snapshot, 11)
        self.assertIn("Topology Trouble Detective", note)
        self.assertNotIn("Rank up", note)

    def test_failures_do_not_overwrite_readme(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root / "README.md").write_text("Missing markers")
            with self.assertRaises(ValueError):
                hall.write_outputs(root, build([issue()]))
            self.assertEqual((root / "README.md").read_text(), "Missing markers")
            self.assertFalse((root / "docs/community/hall-of-fame.json").exists())


class NotificationsTests(unittest.TestCase):
    def test_repository_validation_precedes_network(self):
        with patch.object(hall.subprocess, "run") as network:
            with self.assertRaises(ValueError):
                hall.GitHub("owner/repo?unexpected=query")
            network.assert_not_called()

    def test_paginated_api_response_is_flattened(self):
        with patch.object(hall.subprocess, "run") as network:
            network.return_value.stdout = json.dumps([[{"id": 1}], [{"id": 2}]])
            result = hall.GitHub("example/project").api("repos/example/project/issues", pages=True)
            self.assertEqual(result, [{"id": 1}, {"id": 2}])
            self.assertIn("--paginate", network.call_args.args[0])
            self.assertIn("--slurp", network.call_args.args[0])

    def test_closure_notice_is_not_posted_twice(self):
        snapshot = build([issue(state="closed", reason="completed")])
        snapshot["resolution_awards"]["1"]["notified"] = True
        api = FakeApi(); hall.notify_pending(snapshot, api)
        self.assertEqual(api.posts, [])

    def test_existing_trusted_marker_recovers_interrupted_run(self):
        snapshot = build([issue(state="closed", reason="completed")])
        api = FakeApi([reply(kind="Bot", login="github-actions[bot]", body="<!-- knxbench-field-credit:1 -->")])
        hall.notify_pending(snapshot, api)
        self.assertEqual(api.posts, [])
        self.assertTrue(snapshot["resolution_awards"]["1"]["notified"])

    def test_untrusted_marker_does_not_suppress_credit(self):
        snapshot = build([issue(state="closed", reason="completed")])
        api = FakeApi([reply(body="<!-- knxbench-field-credit:1 -->")])
        hall.notify_pending(snapshot, api)
        self.assertEqual(len(api.posts), 1)
        self.assertTrue(snapshot["resolution_awards"]["1"]["notified"])

    def test_opt_out_does_not_make_an_external_marker_trusted(self):
        for kind in ["User", "Bot"]:
            with self.subTest(kind=kind):
                snapshot = build([issue(state="closed", reason="completed")])
                snapshot["policy"]["excluded_logins"] = POLICY["excluded_logins"] + ["external-agent[bot]"]
                api = FakeApi([reply(kind=kind, login="external-agent[bot]", body="<!-- knxbench-field-credit:1 -->")])
                hall.notify_pending(snapshot, api)
                self.assertEqual(len(api.posts), 1)

    def test_explicitly_trusted_maintainer_marker_recovers_a_notice(self):
        snapshot = build([issue(state="closed", reason="completed")])
        snapshot["policy"]["trusted_notice_logins"] = ["Andre1Becker"]
        api = FakeApi([reply(login="andre1becker", body="<!-- knxbench-field-credit:1 -->")])
        hall.notify_pending(snapshot, api)
        self.assertEqual(api.posts, [])
        self.assertTrue(snapshot["resolution_awards"]["1"]["notified"])

    def test_invalid_trusted_notice_policy_is_refused_before_posting(self):
        for invalid in ["not-a-list", ["bad|handle"]]:
            with self.subTest(invalid=invalid):
                snapshot = build([issue(state="closed", reason="completed")])
                snapshot["policy"]["trusted_notice_logins"] = invalid
                api = FakeApi()
                with self.assertRaises(ValueError):
                    hall.notify_pending(snapshot, api)
                self.assertEqual(api.posts, [])

    def test_external_bot_marker_does_not_suppress_credit(self):
        snapshot = build([issue(state="closed", reason="completed")])
        api = FakeApi([reply(kind="Bot", login="external-agent[bot]", body="<!-- knxbench-field-credit:1 -->")])
        hall.notify_pending(snapshot, api)
        self.assertEqual(len(api.posts), 1)
        self.assertTrue(snapshot["resolution_awards"]["1"]["notified"])

    def test_historical_medal_backfill_does_not_repost_notified_awards(self):
        snapshot = build([issue(i + 1, state="closed", reason="completed") for i in range(20)])
        for award in snapshot["resolution_awards"].values():
            award["notified"] = True
        # Older snapshots had only the ten-report medal. Recompute, do not trust it.
        snapshot["contributors"][0]["medals"] = ["Golden Bus Probe"]
        updated = build([issue(i + 1, state="closed", reason="completed") for i in range(20)], previous=snapshot)
        self.assertEqual(len(updated["contributors"][0]["medals"]), 6)
        api = FakeApi(); hall.notify_pending(updated, api)
        self.assertEqual(api.posts, [])

    def test_reopened_issue_receives_no_misleading_closure_note(self):
        snapshot = build([issue(state="closed", reason="completed")])
        api = FakeApi(); api.state = "open"
        hall.notify_pending(snapshot, api)
        self.assertEqual(api.posts, [])

    def test_write_requires_exact_readback(self):
        snapshot = build([issue(state="closed", reason="completed")])
        api = FakeApi(); api.mismatch = True
        with self.assertRaises(ValueError):
            hall.notify_pending(snapshot, api)
        self.assertFalse(snapshot["resolution_awards"]["1"]["notified"])


class FakeApi:
    def __init__(self, comments=()):
        self.comments = list(comments); self.posts = []; self.state = "closed"; self.mismatch = False

    def get_issue(self, number):
        return issue(number, state=self.state, reason="completed" if self.state == "closed" else None)

    def get_comments(self, number):
        return self.comments

    def post_comment(self, number, body):
        self.posts.append((number, body)); return {"id": 42, "body": body}

    def get_comment(self, cid):
        return {"id": cid, "body": "wrong" if self.mismatch else self.posts[-1][1]}


if __name__ == "__main__":
    unittest.main()
