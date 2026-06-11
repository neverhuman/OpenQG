#!/usr/bin/env python3
"""fig1_architecture: OpenQG V7 engine architecture (double-column, 7.16x3.4)."""
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.patches import FancyBboxPatch, FancyArrowPatch
from matplotlib.path import Path
import os

plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 8,
                     'axes.titlesize': 9, 'axes.labelsize': 8, 'legend.fontsize': 7,
                     'xtick.labelsize': 7, 'ytick.labelsize': 7,
                     'axes.spines.top': False, 'axes.spines.right': False,
                     'axes.linewidth': 0.7, 'figure.dpi': 110})

BLUE, ORANGE, GREEN, VERM, PURPLE, SKY, GRAY = ('#0173B2', '#DE8F05', '#029E73',
                                                '#D55E00', '#CC78BC', '#56B4E9', '#949494')
INK = '#1a1a1a'

fig = plt.figure(figsize=(7.16, 3.4))
ax = fig.add_axes([0, 0, 1, 1])
ax.set_xlim(-1, 101)
ax.set_ylim(0, 100)
ax.axis('off')

boxes = []  # (x, y, w, h, [text artists]) for overflow checks


def box(x, y, w, h, lines, ec, fc='white', lw=0.9, fs=6.2, fst=6.6, rs=0.9):
    p = FancyBboxPatch((x, y), w, h, boxstyle=f"round,pad=0,rounding_size={rs}",
                       fc=fc, ec=ec, lw=lw, zorder=2)
    ax.add_patch(p)
    n = len(lines)
    ts = []
    for i, (s, bold) in enumerate(lines):
        ty = y + h - (i + 0.5) * h / n
        t = ax.text(x + w / 2, ty, s, ha='center', va='center', zorder=4,
                    fontsize=fst if bold else fs,
                    fontweight='bold' if bold else 'normal', color=INK)
        ts.append(t)
    boxes.append((x, y, w, h, ts))


def arr(p0, p1, color='#555555', lw=0.9, rad=0.0, ms=7):
    ax.add_patch(FancyArrowPatch(p0, p1, arrowstyle='-|>', mutation_scale=ms,
                                 lw=lw, color=color, shrinkA=0, shrinkB=0,
                                 connectionstyle=f"arc3,rad={rad}", zorder=3))


def parr(verts, color='#555555', lw=0.9, ms=7, ls='solid'):
    ax.add_patch(FancyArrowPatch(path=Path(verts), arrowstyle='-|>',
                                 mutation_scale=ms, lw=lw, color=color,
                                 linestyle=ls, zorder=3))


# ---- band headers -----------------------------------------------------------
for cx, name, col in [(14, 'PROPOSAL', BLUE), (39.5, 'ORACLE', BLUE),
                      (65, 'EVOLUTION', GREEN), (88.75, 'AUDIT', PURPLE)]:
    ax.text(cx, 97.3, name, ha='center', va='center', fontsize=7,
            fontweight='bold', color=col, zorder=4)

# ---- band 2 background (oracle core, emphasized) ----------------------------
ax.add_patch(FancyBboxPatch((26.3, 10.5), 52.5 - 26.3, 91.5 - 10.5,
                            boxstyle="round,pad=0,rounding_size=1.2",
                            fc=BLUE, alpha=0.08, ec='none', zorder=0.5))
ax.add_patch(FancyBboxPatch((26.3, 10.5), 52.5 - 26.3, 91.5 - 10.5,
                            boxstyle="round,pad=0,rounding_size=1.2",
                            fc='none', ec=BLUE, alpha=0.35, lw=0.7, zorder=0.6))

# ---- band 1: PROPOSAL (x 3..25) ---------------------------------------------
box(3, 78, 22, 8.5, [("jnoccio-fusion router", True),
                     ("131 models / 16 providers", False)], BLUE)
box(3, 60, 22, 11.5, [("RouterProposer", True),
                      ("best-of-K = 4,", False),
                      ("strict JSON schema", False)], BLUE)
arr((14, 78), (14, 71.5), color=BLUE)

# parse + oracle-repair self loop on proposer bottom edge
ax.add_patch(FancyArrowPatch((9, 60), (15.5, 60), arrowstyle='-|>',
                             mutation_scale=6, lw=0.8, color=GRAY,
                             connectionstyle="arc3,rad=-0.85", zorder=3))
t_loop = ax.text(14, 54.6, "parse + oracle repair (redacted)", ha='center',
                 va='center', fontsize=6.2, style='italic', color='#666666',
                 zorder=4)

# lane chips (2 x 2)
chip_lines = [("planck_mu0", 3, 44), ("dark_scattering", 14.4, 44),
              ("free", 3, 38), ("null_diagnostic", 14.4, 38)]
for s, cx0, cy0 in chip_lines:
    p = FancyBboxPatch((cx0, cy0), 10.6, 4.5, boxstyle="round,pad=0,rounding_size=0.7",
                       fc='#eaf4fb', ec=SKY, lw=0.8, zorder=2)
    ax.add_patch(p)
    t = ax.text(cx0 + 5.3, cy0 + 2.25, s, ha='center', va='center',
                fontsize=6.2, color='#0b5a86', zorder=4)
    boxes.append((cx0, cy0, 10.6, 4.5, [t]))
ax.text(14, 50.6, "lanes", ha='center', va='center', fontsize=6.2,
        color='#666666', style='italic', zorder=4)

# ---- band 2: ORACLE (boxes x 27.5..51.5) ------------------------------------
box(27.5, 79.5, 24, 8.5, [("ProposalSketch expander", True),
                          ("(terms follow dials)", False)], BLUE, lw=1.0)
box(27.5, 67, 24, 8.5, [("Truth binding", True),
                        ("cert → integrated background", False)], BLUE, lw=1.0)
box(27.5, 48.5, 24, 14.5, [("physics_kills", True),
                           ("= cascade + adjudication", False),
                           ("(term registry, screening,", False),
                           ("GW170817, PhantomDrag)", False)], BLUE, lw=1.0)
box(27.5, 30, 24, 14.5, [("ScorecardV4 (100 pts)", True),
                         ("rigor 20 | data 20 | novelty 20", False),
                         ("unification 15 | robustness 13", False),
                         ("parsimony 12", False)], BLUE, lw=1.0)
box(27.5, 14.5, 24, 11.5, [("Evidence: ΔLL − 0.5 k ln(n_eff)", True),
                           ("covariance blocks,", False),
                           ("anchor-calibrated CMB", False)], BLUE, lw=1.0)

arr((25, 67), (27.5, 82), color=BLUE, rad=-0.2)        # proposer -> expander
arr((39.5, 79.5), (39.5, 75.5), color=BLUE)            # expander -> binding
arr((39.5, 67), (39.5, 63), color=BLUE)                # binding -> kills
arr((39.5, 48.5), (39.5, 44.5), color=BLUE)            # kills -> scorecard
arr((39.5, 26), (39.5, 30), color=GRAY, lw=0.8, ms=6)  # evidence -> scorecard

# ---- band 3: EVOLUTION (x 55..75) -------------------------------------------
box(55, 79.5, 20, 8.5, [("Population (islands:", True),
                        ("explore / exploit / novelty)", False)], GREEN)
box(55, 65, 20, 8.5, [("Re-clothe operator", True),
                      ("(structure × fit, re-verified)", False)], GREEN)
box(55, 50.5, 20, 8.5, [("Cross-run MEMORY", True),
                        ("(promotable-gated)", False)], GREEN)
arr((65, 79.5), (65, 73.5), color=GREEN)
arr((65, 65), (65, 59), color=GREEN)

# scorecard -> evolution (next band)
parr([(51.5, 37.25), (53.5, 37.25), (53.5, 83.75), (55, 83.75)],
     color='#555555', lw=1.0)

# evolution -> proposer ("next slot", over the top)
parr([(65, 88), (65, 93.2), (1.5, 93.2), (1.5, 65.75), (3, 65.75)],
     color=GREEN, lw=1.0)
ax.text(25, 95.0, "next slot", ha='center', va='center', fontsize=6.2,
        color=GREEN, style='italic', zorder=4)

# ---- band 4: AUDIT (x 78..99.5) ---------------------------------------------
box(78, 76.5, 21.5, 11.5, [("Streaming ledgers", True),
                           ("(attempts, proposals,", False),
                           ("checkpoints)", False)], PURPLE)
box(78, 64, 21.5, 6, [("Replay (0 mismatches)", True)], PURPLE)
box(78, 46, 21.5, 11.5, [("Adversarial audits", True),
                         ("21-agent workflow +", False),
                         ("12-review external referee", False)], PURPLE)
arr((88.75, 76.5), (88.75, 70), color=PURPLE)
arr((88.75, 64), (88.75, 57.5), color=PURPLE)

# everything -> ledgers (gray dashed bus between bands 3 and 4)
parr([(51.5, 20.25), (76.4, 20.25), (76.4, 82.25), (78, 82.25)],
     color=GRAY, lw=0.8, ms=6, ls=(0, (2.2, 1.6)))
ax.plot([75, 76.4], [54.75, 54.75], color=GRAY, lw=0.8,
        ls=(0, (2.2, 1.6)), zorder=3)

# audits -> oracle: the cascade feedback arrow
parr([(88.75, 46), (88.75, 4.5), (39.5, 4.5), (39.5, 10.5)],
     color=VERM, lw=2.0, ms=9)
ax.text(64, 7.6, "each exploit → permanent regression test", ha='center',
        va='center', fontsize=6.4, color=VERM, fontweight='bold', zorder=4)

# ---- sanity checks: text overflow + pairwise overlap ------------------------
fig.canvas.draw()
ren = fig.canvas.get_renderer()
warn = 0
for (x, y, w, h, ts) in boxes:
    (X0, Y0), (X1, Y1) = ax.transData.transform([(x, y), (x + w, y + h)])
    for t in ts:
        bb = t.get_window_extent(ren)
        if bb.x0 < X0 + 1 or bb.x1 > X1 - 1:
            print(f"OVERFLOW: {t.get_text()!r} text {bb.width:.0f}px vs box {(X1-X0):.0f}px")
            warn += 1
texts = [a for a in ax.texts]
for i in range(len(texts)):
    for j in range(i + 1, len(texts)):
        b1, b2 = texts[i].get_window_extent(ren), texts[j].get_window_extent(ren)
        if (b1.x0 < b2.x1 and b2.x0 < b1.x1 and b1.y0 < b2.y1 and b2.y0 < b1.y1):
            print(f"TEXT OVERLAP: {texts[i].get_text()!r} <-> {texts[j].get_text()!r}")
            warn += 1
print(f"checks done, {warn} warnings")

# ---- save -------------------------------------------------------------------
outdir = '/home/ubuntu/openQG/paper/figs'
os.makedirs(outdir, exist_ok=True)
pdf = os.path.join(outdir, 'fig1_architecture.pdf')
png = os.path.join(outdir, 'fig1_architecture.png')
plt.savefig(pdf, bbox_inches='tight')
plt.savefig(png, bbox_inches='tight', dpi=200)
for f in (pdf, png):
    assert os.path.exists(f) and os.path.getsize(f) > 0, f
    print(f, os.path.getsize(f), 'bytes')
