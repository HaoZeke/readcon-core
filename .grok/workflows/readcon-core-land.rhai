let meta = #{
    name: "readcon-core-land",
    description: "Two isolated implementers, then a 2-voter Borda red-team, then apply the winner",
    when_to_use: "Land one claimed readcon-core ticket with 2 implementers and at least 2 reviewers",
    phases: [
        #{ title: "Implement", detail: "two isolated implementers" },
        #{ title: "Borda", detail: "two red-team reviewers rank A vs B" },
        #{ title: "Apply", detail: "merge the Borda winner into the checkout" },
        #{ title: "Review", detail: "two red-team accept votes after apply" },
    ],
};

let impl_schema = #{
    "type": "object",
    "required": ["ok", "summary", "verify", "files", "worktree", "sha"],
    "properties": #{
        "ok": #{ "type": "boolean" },
        "summary": #{ "type": "string" },
        "verify": #{ "type": "string" },
        "files": #{ "type": "string" },
        "worktree": #{ "type": "string" },
        "sha": #{ "type": "string" },
    },
};

let rank_schema = #{
    "type": "object",
    "required": ["first", "second", "evidence", "gaps"],
    "properties": #{
        "first": #{ "type": "string" },
        "second": #{ "type": "string" },
        "evidence": #{ "type": "string" },
        "gaps": #{ "type": "string" },
    },
};

let apply_schema = #{
    "type": "object",
    "required": ["ok", "sha", "summary"],
    "properties": #{
        "ok": #{ "type": "boolean" },
        "sha": #{ "type": "string" },
        "summary": #{ "type": "string" },
    },
};

let vote_schema = #{
    "type": "object",
    "required": ["accept", "evidence", "gaps"],
    "properties": #{
        "accept": #{ "type": "boolean" },
        "evidence": #{ "type": "string" },
        "gaps": #{ "type": "string" },
    },
};

if args == () { pause("verification", "Pass args.ticket and args.root."); }
let ticket = args.ticket;
let root = args.root;
if ticket == () || ticket == "" { pause("verification", "Pass args.ticket — existing readcon-core issue id."); }
if root == () || root == "" { pause("verification", "Pass args.root — readcon-core checkout."); }
let brief = "";
if args.brief != () && args.brief != "" { brief = args.brief; }

fn hard() {
    let s = "";
    s += "HARD CONSTRAINTS:\n";
    s += "- NEVER cargo build / cargo test / cmake / meson / ninja / make locally. Builds and tests only on rg.terra.\n";
    s += "- NEVER rm outside /tmp; use rtrash. NEVER gio trash.\n";
    s += "- No AI attribution. No public GH/GL issue or PR writes.\n";
    s += "- Conventional commits as you go. Push HaoZeke feature branch only, never origin/main (lode-org).\n";
    s += "- Do not mark the ticket DONE. Parent does that after review is unanimous clean.\n";
    s += "- File reviewer defects as children of the ticket BEFORE voting.\n";
    s += "- accept=true only with file:line evidence, zero children filed this turn, empty gaps.\n";
    s
}

fn impl_prompt(which, ticket, root, brief) {
    let p = "";
    p += "You are implementer ";
    p += which;
    p += " of 2 for ";
    p += ticket;
    p += ".\n";
    p += hard();
    p += "Checkout root (do not write here unless isolation failed): ";
    p += root;
    p += "\n";
    p += "isolation_worktree is on. Work only in THIS process cwd. Run pwd and git rev-parse --show-toplevel --abbrev-ref HEAD.\n";
    p += "Read the claimed ticket ";
    p += ticket;
    p += " from the project issue tracker and the acceptance brief below.\n";
    p += "Ticket is already STARTED. Do not invent sibling issue ids.\n";
    p += "TDD: write the failing regression first, then the code. ASCII present-tense comments only.\n";
    p += "Commit on this worktree. Do not merge into the parent checkout. Do not touch unrelated tickets.\n";
    p += "Return ok, summary, verify (exact rg.terra command), files, worktree (abs path of git root), sha (HEAD).\n";
    if brief != "" {
        p += "Acceptance brief: ";
        p += brief;
        p += "\n";
    }
    p
}

phase("Implement");
let impl_jobs = [];
impl_jobs.push(#{
    prompt: impl_prompt("A", ticket, root, brief),
    label: "impl-A",
    capability_mode: "all",
    isolation_worktree: true,
    output_schema: impl_schema,
});
impl_jobs.push(#{
    prompt: impl_prompt("B", ticket, root, brief),
    label: "impl-B",
    capability_mode: "all",
    isolation_worktree: true,
    output_schema: impl_schema,
});
let impls = parallel(impl_jobs);
let a_ok = impls[0] != () && impls[0].success && impls[0].output.ok == true;
let b_ok = impls[1] != () && impls[1].success && impls[1].output.ok == true;
if !a_ok && !b_ok {
    let rpt0 = "# readcon-core-land\n\nticket=" + ticket + "\nboth implementers failed\n";
    let p0 = write_scratch_file("readcon-core-land.md", rpt0);
    complete(#{ path: p0, committed: false, ticket: ticket, stage: "implement" });
}

phase("Borda");
let sum_a = "";
let sum_b = "";
let tree_a = "";
let tree_b = "";
let sha_a = "";
let sha_b = "";
if impls[0] != () && impls[0].success && impls[0].output != () {
    if impls[0].output.summary != () { sum_a = impls[0].output.summary; }
    if impls[0].output.worktree != () { tree_a = impls[0].output.worktree; }
    if impls[0].output.sha != () { sha_a = impls[0].output.sha; }
}
if impls[1] != () && impls[1].success && impls[1].output != () {
    if impls[1].output.summary != () { sum_b = impls[1].output.summary; }
    if impls[1].output.worktree != () { tree_b = impls[1].output.worktree; }
    if impls[1].output.sha != () { sha_b = impls[1].output.sha; }
}
let rank_jobs = [];
let ri = 1;
while ri <= 2 {
    let rp = "";
    rp += "You are red-team Borda voter ";
    rp += ri.to_string();
    rp += " of 2 for ";
    rp += ticket;
    rp += ".\n";
    rp += hard();
    rp += "Read-only. Rank the two isolated implementations. first and second must each be exactly A, B, or neither.\n";
    rp += "Borda score is k - position (first=1, second=0). Vote the independently correct tree first.\n";
    rp += "A worktree=";
    rp += tree_a;
    rp += " sha=";
    rp += sha_a;
    rp += " summary=";
    rp += json_encode(sum_a);
    rp += "\n";
    rp += "B worktree=";
    rp += tree_b;
    rp += " sha=";
    rp += sha_b;
    rp += " summary=";
    rp += json_encode(sum_b);
    rp += "\n";
    rp += "Read ticket ";
    rp += ticket;
    rp += " and the winner-candidate files with read_file/grep. Do not vote from summaries alone.\n";
    rp += "File EVERY defect as a child of ";
    rp += ticket;
    rp += " BEFORE you vote, including nits. If you cannot mint a child issue, put TITLE :: file:line :: verify in gaps.\n";
    rp += "If neither implementation independently satisfies the ticket verify line, first=neither and accept is not used here.\n";
    if brief != "" {
        rp += "Acceptance brief: ";
        rp += brief;
        rp += "\n";
    }
    rank_jobs.push(#{
        prompt: rp,
        label: "borda-" + ri.to_string(),
        capability_mode: "read-only",
        output_schema: rank_schema,
    });
    ri += 1;
}
let ranks = parallel(rank_jobs);
let score_a = 0;
let score_b = 0;
let seen_r = 0;
for r in ranks {
    if r != () && r.success && r.output.first != () {
        seen_r += 1;
        if r.output.first == "A" { score_a += 1; }
        if r.output.first == "B" { score_b += 1; }
    }
}
let winner = "neither";
if score_a > score_b && a_ok { winner = "A"; }
if score_b > score_a && b_ok { winner = "B"; }
if score_a == score_b && a_ok && !b_ok { winner = "A"; }
if score_a == score_b && b_ok && !a_ok { winner = "B"; }
log("borda A=" + score_a.to_string() + " B=" + score_b.to_string() + " winner=" + winner + " seen=" + seen_r.to_string());
if winner == "neither" {
    let rpt1 = "# readcon-core-land\n\nticket=" + ticket + "\nborda winner=neither\n";
    rpt1 += "scoreA=" + score_a.to_string() + " scoreB=" + score_b.to_string() + " seen=" + seen_r.to_string() + "\n";
    let p1 = write_scratch_file("readcon-core-land.md", rpt1);
    complete(#{ path: p1, committed: false, ticket: ticket, stage: "borda", score_a: score_a, score_b: score_b });
}

phase("Apply");
let win_sum = sum_a;
let win_tree = tree_a;
let win_sha = sha_a;
if winner == "B" {
    win_sum = sum_b;
    win_tree = tree_b;
    win_sha = sha_b;
}
let ap = "";
ap += "Apply Borda winner ";
ap += winner;
ap += " for ";
ap += ticket;
ap += " into ";
ap += root;
ap += ".\n";
ap += hard();
ap += "Winner worktree=";
ap += win_tree;
ap += " sha=";
ap += win_sha;
ap += "\nWinner summary: ";
ap += json_encode(win_sum);
ap += "\n";
ap += "Cherry-pick or copy only the winner's committed files into ";
ap += root;
ap += ". Do not invent new behavior. Conventional commit. Do not mark DONE.\n";
ap += "Push HaoZeke feature branch feat/";
ap += ticket;
ap += " if the head is reviewable. Never push origin/main.\n";
let applied = agent(ap, #{
    label: "apply-winner",
    capability_mode: "all",
    output_schema: apply_schema,
});
let applied_ok = applied != () && applied.success && applied.output.ok == true;
if !applied_ok {
    let rpt2 = "# readcon-core-land\n\nticket=" + ticket + "\napply failed\n";
    let p2 = write_scratch_file("readcon-core-land.md", rpt2);
    complete(#{ path: p2, committed: false, ticket: ticket, stage: "apply" });
}

phase("Review");
let review_jobs = [];
let li = 1;
while li <= 2 {
    let lp = "";
    lp += "You are red-team land reviewer ";
    lp += li.to_string();
    lp += " of 2 for ";
    lp += ticket;
    lp += ".\n";
    lp += hard();
    lp += "Read-only. Checkout ";
    lp += root;
    lp += ".\n";
    lp += "Read ticket ";
    lp += ticket;
    lp += " and its child issues from the project tracker.\n";
    lp += "Grep and read_file the shipped files. Confirm the ticket verify line with file:line.\n";
    lp += "File EVERY leftover defect as a child of ";
    lp += ticket;
    lp += " BEFORE you vote, including nits.\n";
    lp += "accept=true only with empty gaps and no newly filed children.\n";
    if brief != "" {
        lp += "Acceptance brief: ";
        lp += brief;
        lp += "\n";
    }
    review_jobs.push(#{
        prompt: lp,
        label: "rev-" + li.to_string(),
        capability_mode: "read-only",
        output_schema: vote_schema,
    });
    li += 1;
}
let votes = parallel(review_jobs);
let yes = 0;
let seen = 0;
let gaps = "";
for v in votes {
    if v != () && v.success && v.output.accept != () {
        seen += 1;
        let g = v.output.gaps;
        let clean = g == () || g == "";
        if v.output.accept == true && clean { yes += 1; }
        if !clean {
            gaps += g;
            gaps += "\n";
        }
    }
}
let committed = applied_ok && yes == 2 && seen >= 2;
log("review " + yes.to_string() + "/" + seen.to_string() + " committed=" + committed.to_string());
let rpt = "# readcon-core-land\n\n";
rpt += "ticket=" + ticket + "\n";
rpt += "winner=" + winner + " bordaA=" + score_a.to_string() + " bordaB=" + score_b.to_string() + "\n";
rpt += "review=" + yes.to_string() + "/" + seen.to_string() + "\n";
rpt += "committed=" + committed.to_string() + "\n";
rpt += gaps;
let pth = write_scratch_file("readcon-core-land.md", rpt);
complete(#{ path: pth, committed: committed, ticket: ticket, yes: yes, seen: seen, winner: winner });
