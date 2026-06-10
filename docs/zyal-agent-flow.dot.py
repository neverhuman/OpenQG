#!/usr/bin/env python3
"""High-quality single-page agent-flow diagram (Graphviz, HTML-table swimlanes).

Each of the three lanes is one HTML-table node (a clean grid); cross-lane flow is
drawn with port edges. Grounded in the real OpenQG ZYAL / jailgun code paths.
"""
import subprocess

OUT_DOT = "/home/ubuntu/openQG/docs/zyal-agent-flow.dot"
OUT_PDF = "/home/ubuntu/openQG/docs/zyal-agent-flow.pdf"
OUT_PNG = "/tmp/zyal-agent-flow.png"

HARD = "#fbe3d6"; SOFT = "#eef1f3"; BACK = "#dcefe3"; GPT = "#ece3fb"
SCORE = "#dcf0f2"; ADV = "#f6dce8"; REC = "#f7edd2"; OUT = "#e6ebf1"; ORCH = "#dce6f4"


def row(port, bg, title, detail="", live=False, num=None):
    badge = ' &#160;<FONT COLOR="#c1440e" POINT-SIZE="9"><B>&#9679; LIVE</B></FONT>' if live else ""
    numhtml = f'<FONT COLOR="#c1440e"><B>{num}</B></FONT> &#160;' if num else ""
    det = f'<BR/><FONT POINT-SIZE="9" COLOR="#334155">{detail}</FONT>' if detail else ""
    return (f'    <TR><TD PORT="{port}" BGCOLOR="{bg}" ALIGN="LEFT" BALIGN="LEFT">'
            f'{numhtml}<B>{title}</B>{badge}{det}</TD></TR>\n')


def table(border_color, header, rows, width=240):
    h = (f'<<TABLE BORDER="0" CELLBORDER="1" CELLSPACING="0" CELLPADDING="7" '
         f'COLOR="{border_color}" WIDTH="{width}">\n'
         f'    <TR><TD BGCOLOR="{border_color}"><FONT COLOR="white" POINT-SIZE="13"><B>{header}</B></FONT></TD></TR>\n')
    return h + "".join(rows) + '  </TABLE>>'


# ---- lane 1: pipeline ----
pipe_rows = [row("orch", ORCH, "ZYAL Genome Orchestrator",
                 "preflight: jailgun /api/health &#183; accounts &#183; MCP tools &#183; routing")]
stages = [
    ("s00", "00 &#183; atlas", "research synthesis", True),
    ("s01", "01 &#183; decompose-known", "split solved structure", False),
    ("s02", "02 &#183; decompose-failed", "hard-stage repair", True),
    ("s03", "03 &#183; generate-genes", "synthesize candidate genes", False),
    ("s04", "04 &#183; repair-genes", "hard-stage repair", True),
    ("s05", "05 &#183; compatibility", "hard-stage repair", True),
    ("s06", "06 &#183; assemble-modules", "compose theory modules", False),
    ("s07", "07 &#183; macro-test", "score vs observables", False),
    ("s08", "08 &#183; failure-slicing", "hard-stage repair", True),
    ("s09", "09 &#183; selection-mutation", "hard-stage repair", True),
    ("s10", "10 &#183; promotion", "promotion judging", True),
]
for sid, t, d, hard in stages:
    pipe_rows.append(row(sid, HARD if hard else SOFT, t, d, live=hard))
pipe = table("#1f3a5f", "Per-generation pipeline", pipe_rows, 250)

# ---- lane 2: live backend ----
back_rows = [
    row("b1", BACK, "openqg-bench builds prompt", "prompt.md &#183; prompt_ref &#183; ARTIFACT_REPAIR_ATTEMPTS=0", num="1"),
    row("b2", BACK, "MCP &#8594; jailgun.run", "POST /mcp &#183; JSON-RPC tools/call &#183; tabs=1", num="2"),
    row("b3", BACK, "jailgun-server orchestrator", "lease account &#183; spawn run &#183; Hello&#8594;BridgeReady", num="3"),
    row("b4", BACK, "chrome-bridge (Node+Playwright)", "connectOverCDP 127.0.0.1:9224/9225 (Xvfb :99)", num="4"),
    row("b5", SOFT, "Managed Chrome &#8212; authenticated", "persistent profile acct-19ae9aed / acct-4690d657", num="5"),
    row("b6", GPT, "ChatGPT (chatgpt.com)", "submit prompt &#8594; poll completion &#183; pro-extended", num="6"),
    row("b7", BACK, "capture &#8594; parse", "raw-output &#8594; parsed-summary.json (ok | timeout)", num="7"),
    row("b8", REC, "seal call &#8594; ledgers", "receipt.json &#183; live-call-ledger.jsonl", num="8"),
    row("bfb", SOFT, "fallback (soft / not live)", "scripted-wrapper &#183; jnoccio (top-20% band)"),
]
back = table("#1f7a4d", "LIVE jailgun backend &#183; per hard stage", back_rows, 250)

# ---- lane 3: eval ----
eval_rows = [
    row("e1", SCORE, "Scoring &amp; MAP-Elites QD archive",
        "local &#183; interface &#183; macro &#183; innovation &#183; novelty &#8722; failure_penalty<BR/>islands (diversity) &#183; novelty archive &#183; Pareto frontier"),
    row("e2", ADV, "Adversary / Judge (co-evolving)",
        "frontier: GrayBox &#183; Overfit &#183; Unfalsifiable<BR/>honesty rollback (anchor survival floor) &#183; live critic lowers-only"),
    row("e3", REC, "ProposalReceipt &#183; sealed (M6)",
        "sha256(model output) + oracle verdict<BR/>parse &#8594; derivation-check &#8594; veto cascade &#183; replay without model"),
    row("e4", OUT, "Selection &#8594; Promotion",
        "champion per generation (promote_lineage)<BR/>promotion-ledger / generation-ledger &#183; quality gate"),
    row("e5", OUT, "Outputs / Evidence (per run)",
        "run-summary &#183; metrics-timeseries<BR/>live-call-ledger (1 record / real ChatGPT call)<BR/>quality-gate &#183; pareto-snapshot &#183; champion"),
]
ev = table("#334155", "Scoring &#183; adversary &#183; promotion &#183; outputs", eval_rows, 270)

# ---- legend ----
leg_cells = [("Orchestrator", ORCH), ("Hard stage &#8594; LIVE", HARD), ("Deterministic", SOFT),
             ("Jailgun backend", BACK), ("ChatGPT", GPT), ("Scoring/archive", SCORE),
             ("Adversary/judge", ADV), ("Receipts (M6)", REC), ("Outputs", OUT)]
legtds = "".join(f'<TD BGCOLOR="{c}"><FONT POINT-SIZE="10">{n}</FONT></TD>' for n, c in leg_cells)
legend = (f'<<TABLE BORDER="0" CELLBORDER="1" CELLSPACING="0" CELLPADDING="6" COLOR="#94a3b8">'
          f'<TR><TD BGCOLOR="#475569"><FONT COLOR="white"><B>Legend</B></FONT></TD>{legtds}'
          f'<TD><FONT POINT-SIZE="9">solid = control/data flow &#160; dashed = feedback/seal &#160; dotted = generation loop</FONT></TD></TR></TABLE>>')

dot = f'''digraph zyal {{
  rankdir=TB; bgcolor="white"; compound=true; nodesep=0.9; ranksep=0.7;
  graph [fontname="Helvetica", margin=0.25];
  node  [shape=plaintext, fontname="Helvetica"];
  edge  [fontname="Helvetica", penwidth=1.8, arrowsize=0.9, fontsize=10];
  labelloc="t"; fontsize=23; fontname="Helvetica-Bold";
  label=<<FONT POINT-SIZE="23"><B>OpenQG &#183; ZYAL Genome &#8212; Agent Flow</B></FONT><BR/><FONT POINT-SIZE="12" COLOR="#334155">variant: jailgun-only &#160;&#183;&#160; mode: live-selective &#160;&#183;&#160; real ChatGPT via authenticated browser (CDP) &#160;&#183;&#160; M6 sealed-nondeterminism receipts</FONT><BR/> >;

  pipe [label={pipe}];
  back [label={back}];
  eval [label={ev}];
  legend [label={legend}];

  {{ rank=same; pipe; back; eval; }}
  pipe -> back [style=invis];
  back -> eval [style=invis];

  // cross-lane control / data flow
  pipe:s04:e -> back:b1:w [color="#c1440e", xlabel="  every LIVE stage\\n  calls backend  ", fontcolor="#c1440e"];
  back:b8:e  -> eval:e1:w [color="#1f7a4d", xlabel="  parsed score  ", fontcolor="#1f7a4d"];
  back:b8:e  -> eval:e3:w [color="#9a6b00", style=dashed];
  eval:e2:e  -> eval:e1:e [color="#9a1750", style=dashed, xlabel=" feeds\\n selection "];
  eval:e4:w  -> pipe:orch:e [color="#1f3a5f", style=dotted, xlabel=" loop: next generation\\n (islands &#183; novelty) "];

  pipe -> legend [style=invis];
  back -> legend [style=invis];
  eval -> legend [style=invis];
}}
'''

with open(OUT_DOT, "w") as f:
    f.write(dot)
subprocess.run(["dot", "-Tpdf", "-o", OUT_PDF, OUT_DOT], check=True)
subprocess.run(["dot", "-Tpng", "-Gdpi=130", "-o", OUT_PNG, OUT_DOT], check=True)
print("wrote", OUT_PDF)
