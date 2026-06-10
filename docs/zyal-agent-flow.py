#!/usr/bin/env python3
"""Single-page agent-flow diagram for the OpenQG ZYAL genome (jailgun-only, live).

Renders a dense but readable one-page landscape PDF documenting the real control
flow traced from the codebase: the per-generation 11-stage pipeline, the live
jailgun→ChatGPT backend round-trip for hard stages, scoring / MAP-Elites archive,
the co-evolving adversary/judge, sealed-nondeterminism receipts, and outputs.
"""
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import FancyBboxPatch, FancyArrowPatch, Rectangle
from matplotlib.lines import Line2D

# ---- palette -------------------------------------------------------------
C = {
    "orch":   "#1f3a5f",  # orchestrator (deep blue)
    "orch_b": "#dce6f4",
    "hard":   "#c1440e",  # hard / live stage (burnt orange)
    "hard_b": "#fbe3d6",
    "soft":   "#6b7280",  # deterministic stage (gray)
    "soft_b": "#eef0f2",
    "jail":   "#1f7a4d",  # jailgun backend (green)
    "jail_b": "#dcefe3",
    "gpt":    "#6d28d9",  # ChatGPT (purple)
    "gpt_b":  "#ece3fb",
    "score":  "#0e6e7a",  # scoring / archive (teal)
    "score_b":"#dceff1",
    "adv":    "#9a1750",  # adversary/judge (magenta)
    "adv_b":  "#f6dce8",
    "rec":    "#9a6b00",  # receipts (gold)
    "rec_b":  "#f6ecd0",
    "out":    "#334155",  # outputs (slate)
    "out_b":  "#e2e8f0",
    "ink":    "#111827",
    "arrow":  "#374151",
    "live":   "#c1440e",
}

fig = plt.figure(figsize=(17.0, 9.55))
ax = fig.add_axes([0, 0, 1, 1])
ax.set_xlim(0, 100)
ax.set_ylim(0, 56)
ax.axis("off")
ax.add_patch(Rectangle((0, 0), 100, 56, facecolor="white", edgecolor="none", zorder=-10))


def box(x, y, w, h, title, body="", fc="#ffffff", ec="#333333", tc="#111827",
        tfs=8.2, bfs=6.6, bold=True, round=0.10, lw=1.1, align="center", z=3):
    p = FancyBboxPatch((x, y), w, h, boxstyle=f"round,pad=0.02,rounding_size={round}",
                       linewidth=lw, edgecolor=ec, facecolor=fc, zorder=z)
    ax.add_patch(p)
    cx = x + w / 2 if align == "center" else x + 0.55
    ha = "center" if align == "center" else "left"
    if title and body:
        ax.text(cx, y + h - 0.95, title, ha=ha, va="top", fontsize=tfs,
                fontweight="bold" if bold else "normal", color=tc, zorder=z + 1)
        ax.text(cx, y + h - 2.05, body, ha=ha, va="top", fontsize=bfs,
                color="#1f2937", zorder=z + 1, linespacing=1.28)
    else:
        ax.text(x + w / 2, y + h / 2, title, ha="center", va="center", fontsize=tfs,
                fontweight="bold" if bold else "normal", color=tc, zorder=z + 1,
                linespacing=1.25)
    return (x, y, w, h)


def arrow(p1, p2, color=C["arrow"], lw=1.5, style="-|>", rad=0.0, z=2, ls="solid", ms=10):
    a = FancyArrowPatch(p1, p2, arrowstyle=style, mutation_scale=ms, color=color,
                        lw=lw, connectionstyle=f"arc3,rad={rad}", zorder=z,
                        linestyle=ls, shrinkA=1.5, shrinkB=1.5)
    ax.add_patch(a)


def label(x, y, s, fs=6.2, color="#374151", style="italic", ha="center", weight="normal", z=6):
    ax.text(x, y, s, ha=ha, va="center", fontsize=fs, color=color, style=style,
            fontweight=weight, zorder=z)


# ---- header --------------------------------------------------------------
ax.add_patch(Rectangle((0, 51.6), 100, 4.4, facecolor=C["orch"], edgecolor="none", zorder=2))
ax.text(1.6, 54.5, "OpenQG · ZYAL Genome — Agent Flow",
        ha="left", va="center", fontsize=17, fontweight="bold", color="white", zorder=4)
ax.text(1.6, 52.5, "variant: jailgun-only   ·   mode: live-selective   ·   real ChatGPT via authenticated browser (CDP)   ·   sealed-nondeterminism receipts (M6)",
        ha="left", va="center", fontsize=8.2, color="#cdd9ec", zorder=4)
ax.text(98.4, 53.5, "zyal genome run --variant jailgun-only\n--jailgun-available --live-selective",
        ha="right", va="center", fontsize=7.0, color="#aebfd8", family="monospace", zorder=4)

# =========================================================================
# LEFT LANE — orchestrator + per-generation 11-stage pipeline
# =========================================================================
LX, LW = 1.6, 23.2
box(LX, 46.4, LW, 4.4,
    "ZYAL Genome Orchestrator",
    "preflight: jailgun /api/health · accounts ready ·\nMCP tools · token · routing decision",
    fc=C["orch_b"], ec=C["orch"], tfs=9.0, bfs=6.4, align="left")

stages = [
    ("00", "atlas — research synthesis", True),
    ("01", "decompose-known", False),
    ("02", "decompose-failed — repair", True),
    ("03", "generate-genes", False),
    ("04", "repair-genes — repair", True),
    ("05", "compatibility — repair", True),
    ("06", "assemble-modules", False),
    ("07", "macro-test", False),
    ("08", "failure-slicing — repair", True),
    ("09", "selection-mutation — repair", True),
    ("10", "promotion — judging", True),
]
top = 45.3
sh, gap = 3.18, 0.36
stage_y = {}
for i, (num, name, hard) in enumerate(stages):
    y = top - i * (sh + gap) - sh
    stage_y[num] = (y, sh)
    fc = C["hard_b"] if hard else C["soft_b"]
    ec = C["hard"] if hard else C["soft"]
    ax.add_patch(FancyBboxPatch((LX, y), LW, sh, boxstyle="round,pad=0.02,rounding_size=0.08",
                 linewidth=1.3 if hard else 1.0, edgecolor=ec, facecolor=fc, zorder=3))
    ax.text(LX + 1.5, y + sh / 2, num, ha="center", va="center", fontsize=9.5,
            fontweight="bold", color=ec, zorder=4)
    ax.add_line(Line2D([LX + 3.0, LX + 3.0], [y + 0.3, y + sh - 0.3], color=ec, lw=0.8, zorder=4))
    ax.text(LX + 3.7, y + sh / 2, name, ha="left", va="center", fontsize=7.0,
            color=C["ink"], zorder=4)
    if hard:
        ax.text(LX + LW - 0.7, y + sh / 2, "● LIVE", ha="right", va="center", fontsize=5.6,
                fontweight="bold", color=C["hard"], zorder=4)
    if i > 0:
        py = top - (i - 1) * (sh + gap) - sh
        arrow((LX + LW - 4.0, py), (LX + LW - 4.0, y + sh), color=C["soft"], lw=1.1, ms=8)

# orchestrator -> stage 00
arrow((LX + 3, 46.4), (LX + 3, top), color=C["orch"], lw=1.4)
label(13.2, 50.2, "per generation  ▼", fs=6.4, color=C["orch"], style="normal", weight="bold")

# loop back: promotion -> orchestrator (next generation)
y10 = stage_y["10"][0]
arrow((LX + 0.4, y10 + 1.6), (0.8, y10 + 1.6), color=C["orch"], lw=1.4)
ax.add_line(Line2D([0.8, 0.8], [y10 + 1.6, 48.6], color=C["orch"], lw=1.4, zorder=2))
arrow((0.8, 48.6), (LX, 48.6), color=C["orch"], lw=1.4)
ax.text(1.5, 30, "↺  next generation  (islands · novelty · MAP-Elites refresh)",
        rotation=90, ha="center", va="center", fontsize=6.3, color=C["orch"],
        fontweight="bold", zorder=6)

# =========================================================================
# MIDDLE LANE — live jailgun backend (per HARD stage round-trip)
# =========================================================================
MX, MW = 28.2, 30.0
ax.add_patch(FancyBboxPatch((MX - 0.7, 4.2), MW + 1.4, 46.0,
             boxstyle="round,pad=0.02,rounding_size=0.15", linewidth=1.3,
             edgecolor=C["jail"], facecolor="#f3faf6", zorder=1))
ax.text(MX + MW / 2, 49.2, "LIVE  JAILGUN  BACKEND   ·   per hard stage",
        ha="center", va="center", fontsize=9.4, fontweight="bold", color=C["jail"], zorder=3)
ax.text(MX + MW / 2, 47.5, "route when:  stage is hard  ∧  jailgun_available  ∧  live_selective  →  tier top20",
        ha="center", va="center", fontsize=6.3, color="#1f7a4d", style="italic", zorder=3)

mid = [
    ("openqg-bench builds prompt", "prompt.md · prompt_ref · account_ids\nbridge_env: JAILGUN_ARTIFACT_REPAIR_ATTEMPTS=0", C["jail_b"], C["jail"]),
    ("MCP  POST /mcp → jailgun.run", "JSON-RPC tools/call  ·  run_id  ·  tabs=1", "#ffffff", C["jail"]),
    ("jailgun-server orchestrator", "lease browser account · spawn run\nHello → wait BridgeReady (90s)", "#ffffff", C["jail"]),
    ("chrome-bridge  (Node + Playwright)", "NDJSON over stdio · connectOverCDP\n127.0.0.1:9224 / :9225  (Xvfb :99)", "#ffffff", C["jail"]),
    ("Managed Chrome — authenticated", "profile acct-19ae9aed / acct-4690d657\npersistent ChatGPT session", C["soft_b"], C["soft"]),
    ("ChatGPT  (chatgpt.com)", "submit prompt → poll completion\nmodel: pro-extended", C["gpt_b"], C["gpt"]),
    ("capture → parse response", "raw-output.txt → parsed-summary.json\n(status ok | timeout)", "#ffffff", C["jail"]),
    ("seal call → ledgers", "receipt.json  ·  live-call-ledger.jsonl", C["rec_b"], C["rec"]),
]
my = 44.6
mh, mgap = 4.55, 0.62
midboxes = []
for i, (t, b, fc, ec) in enumerate(mid):
    y = my - i * (mh + mgap) - mh
    midboxes.append((y, mh))
    tc = C["gpt"] if "ChatGPT" in t else C["ink"]
    box(MX, y, MW, mh, t, b, fc=fc, ec=ec, tfs=7.6, bfs=6.0, align="center", tc=tc)
    if i > 0:
        py = my - (i - 1) * (mh + mgap) - mh
        arrow((MX + MW / 2, py), (MX + MW / 2, y + mh), color=C["jail"], lw=1.5)

# entry from hard stages (left pipeline) into the backend
arrow((LX + LW + 0.2, 24.5), (MX, midboxes[0][0] + mh / 2), color=C["hard"], lw=1.7, rad=-0.12)
label((LX + LW + MX) / 2 + 0.3, 27.0, "hard stage\ncalls backend", fs=6.0, color=C["hard"], weight="bold")
# return: ledger/score back to pipeline
arrow((MX, midboxes[-1][0] + mh / 2), (LX + LW + 0.2, 9.2), color=C["jail"], lw=1.5, rad=-0.18, ls="dashed")
label((LX + LW + MX) / 2 + 1.0, 6.2, "parsed score →\nback to stage", fs=6.0, color=C["jail"], weight="bold")

# fallback note
ax.add_patch(FancyBboxPatch((MX, 4.5), MW, 2.2, boxstyle="round,pad=0.02,rounding_size=0.08",
             linewidth=1.0, edgecolor=C["soft"], facecolor="#fbfbfb", zorder=3))
ax.text(MX + MW / 2, 5.6, "fallback (soft stage / not live):  scripted-wrapper  ·  jnoccio-fusion (top-20% band)",
        ha="center", va="center", fontsize=6.0, color=C["soft"], style="italic", zorder=4)

# =========================================================================
# RIGHT LANE — scoring / archive, adversary, receipts, promotion, outputs
# =========================================================================
RX, RW = 60.6, 38.0

def panel(y, h, title, body, fc, ec, tfs=8.4):
    box(RX, y, RW, h, title, body, fc=fc, ec=ec, tfs=tfs, bfs=6.4, align="left")
    return (y, h)

p1 = panel(43.0, 7.6, "Scoring  &  MAP-Elites  QD archive",
           "fitness blend:  local · interface · macro · innovation\n"
           "                · novelty  −  failure_penalty\n"
           "islands keep diversity · novelty archive · QD score · Pareto frontier",
           C["score_b"], C["score"])
p2 = panel(33.6, 8.6, "Adversary  /  Judge   (co-evolving)",
           "frontier escalation each gen:  GrayBox · Overfit ·\n"
           "   Unfalsifiable · BelowFrontier\n"
           "honesty rollback if protected anchors die (survival floor)\n"
           "live critic (ZYAL_LIVE_CRITIC, jnoccio) can only LOWER a passed score",
           C["adv_b"], C["adv"])
p3 = panel(24.4, 8.2, "Sealed-nondeterminism · ProposalReceipt (M6)",
           "input_sha256( exact model output )  +  deterministic oracle verdict\n"
           "oracle:  parse → derivation-check (demote unproven) → veto cascade\n"
           "⇒ re-adjudicate / replay a run WITHOUT calling the model",
           C["rec_b"], C["rec"])
p4 = panel(17.2, 6.2, "Selection → Promotion",
           "champion promoted per generation (promote_lineage)\n"
           "→ promotion-ledger / generation-ledger · quality gate",
           C["out_b"], C["out"])
p5 = panel(5.2, 10.0, "Outputs  /  Evidence  (per run)",
           "run-summary.json · metrics-timeseries.jsonl\n"
           "live-call-ledger.jsonl  (one record per real ChatGPT call)\n"
           "quality-gate.json · pareto-snapshot.json · champion\n"
           "stages/<NN>/generations/<gNNNN>/live-calls/.../receipt.json",
           "#f8fafc", C["out"], tfs=8.4)

# connectors within right lane
for (ya, ha_), (yb, hb_) in [(p1, p2), (p2, p4), (p4, p5)]:
    arrow((RX + RW * 0.5, ya), (RX + RW * 0.5, yb + hb_), color=C["score"], lw=1.4)
# adversary <-> archive feedback loop
arrow((RX + RW - 1.5, p2[0] + p2[1]), (RX + RW - 1.5, p1[0] + 0.3),
      color=C["adv"], lw=1.2, rad=-0.4, ls="dashed")
label(RX + RW - 0.2, (p1[0] + p2[0] + p2[1]) / 2 + 1.2, "feeds\nselection", fs=5.6, color=C["adv"],
      ha="right", weight="bold")
# receipts tie to backend ledger
arrow((RX, p3[0] + p3[1] / 2), (MX + MW + 0.6, midboxes[-1][0] + mh / 2),
      color=C["rec"], lw=1.2, rad=0.16, ls="dashed")

# backend score -> scoring panel
arrow((MX + MW + 0.7, midboxes[6][0] + mh / 2), (RX, p1[0] + p1[1] / 2),
      color=C["jail"], lw=1.5, rad=0.10)
# promotion result feeds the next-generation loop (shown on the left lane)
label(RX - 0.4, p4[0] + p4[1] / 2, "↺ next gen", fs=5.6, color=C["orch"], ha="right", weight="bold", style="normal")

# =========================================================================
# legend / footer
# =========================================================================
ax.add_patch(Rectangle((0, 0), 100, 4.0, facecolor="#f1f3f6", edgecolor="none", zorder=1))
leg = [
    ("Orchestrator", C["orch"]), ("Hard stage → LIVE", C["hard"]), ("Deterministic stage", C["soft"]),
    ("Jailgun backend", C["jail"]), ("ChatGPT", C["gpt"]), ("Scoring / archive", C["score"]),
    ("Adversary / judge", C["adv"]), ("Receipts (M6)", C["rec"]), ("Outputs", C["out"]),
]
x = 1.6
for name, col in leg:
    ax.add_patch(Rectangle((x, 1.35), 1.3, 1.3, facecolor=col, edgecolor="none", zorder=4))
    ax.text(x + 1.8, 2.0, name, ha="left", va="center", fontsize=6.8, color="#111827", zorder=4)
    x += 2.3 + len(name) * 0.62
ax.text(98.4, 0.7, "evidence: target/openqg/zyal-genome/jailgun-only/runs/<run-id>/   ·   solid = data/control flow   ·   dashed = feedback   ·   dotted = generation loop",
        ha="right", va="center", fontsize=5.8, color="#6b7280", style="italic", zorder=4)

out = "/home/ubuntu/openQG/docs/zyal-agent-flow.pdf"
fig.savefig(out, format="pdf", bbox_inches=None)
print("wrote", out)
