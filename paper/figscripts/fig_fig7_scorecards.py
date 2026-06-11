#!/usr/bin/env python3
"""fig7_scorecards: horizontal stacked component bars for five scorecards."""
import json
import os

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.patches import Patch

plt.rcParams.update({
    'font.family': 'DejaVu Sans', 'font.size': 8, 'axes.titlesize': 9,
    'axes.labelsize': 8, 'legend.fontsize': 7, 'xtick.labelsize': 7,
    'ytick.labelsize': 7, 'axes.spines.top': False, 'axes.spines.right': False,
    'axes.linewidth': 0.7, 'figure.dpi': 110, 'hatch.linewidth': 0.5,
})

PALETTE = ['#0173B2', '#DE8F05', '#029E73', '#D55E00', '#CC78BC', '#56B4E9',
           '#949494', '#ECE133']

ROOT = '/home/ubuntu/openQG'
with open(os.path.join(ROOT, 'paper', 'data', 'story.json')) as f:
    story = json.load(f)

COMPONENTS = ['derivation_rigor', 'data_fit', 'novel_prediction',
              'unification', 'robustness_under_judge', 'parsimony']
COMP_LABELS = ['Derivation rigor', 'Data fit', 'Novel prediction',
               'Unification', 'Robustness (judge)', 'Parsimony']
COMP_COLORS = PALETTE[:6]

v5 = story['v5_champ_components']
v6 = story['v6_77_components']
v7 = {'derivation_rigor': 6, 'data_fit': 0, 'novel_prediction': 5,
      'unification': 15, 'robustness_under_judge': 13, 'parsimony': 4}
survivor = {'derivation_rigor': 20, 'data_fit': 0, 'novel_prediction': 0,
            'unification': 15, 'robustness_under_judge': 13, 'parsimony': 12}
human = {'derivation_rigor': 25, 'data_fit': 18, 'novel_prediction': 0,
         'unification': 12, 'robustness_under_judge': 13, 'parsimony': 12}

# top-to-bottom: V5 -> V6 -> survivor -> V7 -> human   (barh: top = max y)
bars = [
    # (y, label, components, alpha, hatched)
    (4, 'V5 champion', v5, 1.0, False),
    (3, 'V6 champion', v6, 1.0, False),
    (2, 'V6.1 survivor\n(nDGP)', survivor, 1.0, True),
    (1, 'V7 champion\n(planck-mu0-\nsuppressed-0.1)', v7, 1.0, False),
    (0, 'Human baseline\n(GR+ΛCDM\nprogram)', human, 0.55, False),
]

# sanity: totals
for _, lab, comp, _, _ in bars:
    tot = sum(comp[c] for c in COMPONENTS)
    print(f'{lab.split(chr(10))[0]:>16s}: total = {tot}')

BAR_H = 0.62
fig, ax = plt.subplots(figsize=(3.5, 3.4))

for y, lab, comp, alpha, hatched in bars:
    left = 0.0
    for cname, color in zip(COMPONENTS, COMP_COLORS):
        w = comp[cname]
        if w <= 0:
            left += w
            continue
        ax.barh(y, w, height=BAR_H, left=left, color=color, alpha=alpha,
                edgecolor='white', linewidth=0.4, zorder=2)
        # in-segment value labels (only when wide enough to fit)
        if w >= 8:
            txt_color = '#222222' if alpha < 1.0 else 'white'
            ax.text(left + w / 2.0, y, str(w), ha='center', va='center',
                    fontsize=6, color=txt_color, zorder=4)
        left += w
    total = left
    if hatched:  # DQ overlay
        ax.barh(y, total, height=BAR_H, left=0, facecolor='none',
                edgecolor='#444444', hatch='//', linewidth=0.6, zorder=3)
    ax.text(total + 1.5, y, f'{total:.0f}', ha='left', va='center',
            fontsize=7, fontweight='bold', color='black', zorder=4)

# skull-note on the disqualified survivor
ax.text(60 + 7.5, 2, '☠ DQ under V7:\n   no brane term',
        ha='left', va='center', fontsize=6, color='black', zorder=4)

# note on the human baseline
ax.text(1.0, 0 - BAR_H / 2.0 - 0.16, '(V4-era rubric, indicative)',
        ha='left', va='top', fontsize=6, style='italic', color='#949494')

ax.set_yticks([b[0] for b in bars])
ax.set_yticklabels([b[1] for b in bars])
ax.set_xlim(0, 96)
ax.set_ylim(-0.95, 4.55)
ax.set_xlabel('Rubric score (points)')
ax.tick_params(axis='y', length=0)

handles = [Patch(facecolor=c, edgecolor='none') for c in COMP_COLORS]
fig.legend(handles, COMP_LABELS, loc='lower center',
           bbox_to_anchor=(0.52, 0.0), ncol=3, frameon=False,
           handlelength=1.0, handleheight=0.9, columnspacing=0.7,
           handletextpad=0.35, labelspacing=0.35)

fig.subplots_adjust(left=0.27, right=0.985, top=0.985, bottom=0.27)

out_pdf = os.path.join(ROOT, 'paper', 'figs', 'fig7_scorecards.pdf')
out_png = os.path.join(ROOT, 'paper', 'figs', 'fig7_scorecards.png')
os.makedirs(os.path.dirname(out_pdf), exist_ok=True)
fig.savefig(out_pdf, bbox_inches='tight')
fig.savefig(out_png, bbox_inches='tight', dpi=200)

for p in (out_pdf, out_png):
    sz = os.path.getsize(p)
    assert sz > 0, f'empty file: {p}'
    print(f'{p}  ({sz} bytes)')
