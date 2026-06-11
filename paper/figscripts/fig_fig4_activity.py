#!/usr/bin/env python3
"""fig4_activity: campaign activity raster (V5, V6, V7 first-3-chunks)."""
import json
import os

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import matplotlib.transforms as mtransforms
from matplotlib.lines import Line2D
from matplotlib.cm import ScalarMappable
from matplotlib.colors import Normalize
import numpy as np

plt.rcParams.update({
    'font.family': 'DejaVu Sans', 'font.size': 8, 'axes.titlesize': 9,
    'axes.labelsize': 8, 'legend.fontsize': 7, 'xtick.labelsize': 7,
    'ytick.labelsize': 7, 'axes.spines.top': False, 'axes.spines.right': False,
    'axes.linewidth': 0.7, 'figure.dpi': 110})

PAL = ['#0173B2', '#DE8F05', '#029E73', '#D55E00', '#CC78BC', '#56B4E9',
       '#949494', '#ECE133']

DATA = '/home/ubuntu/openQG/paper/data/campaigns.json'
OUT_PDF = '/home/ubuntu/openQG/paper/figs/fig4_activity.pdf'
OUT_PNG = '/home/ubuntu/openQG/paper/figs/fig4_activity.png'

with open(DATA) as f:
    camps = json.load(f)

# Panel definitions: (title, list of (row_label, chunk_dict))
v7_keys = ['v6-chunk-1', 'v6-chunk-2', 'v6-chunk-3']  # first 3 chunks only
panels = [
    ('V5 (37 proposals)', [(i + 1, camps['v5'][k])
                           for i, k in enumerate(sorted(camps['v5']))]),
    ('V6 (140)', [(i + 1, camps['v6'][k])
                  for i, k in enumerate(sorted(camps['v6']))]),
    ('V7 (partial)', [(i + 1, camps['v7'][k]) for i, k in enumerate(v7_keys)]),
]

ERROR_STATUSES = {'parse_error', 'llm_error', 'empty_output', 'engine_error'}

rng = np.random.default_rng(42)
norm = Normalize(vmin=0, vmax=80)
cmap = plt.get_cmap('viridis')

fig, axes = plt.subplots(1, 3, figsize=(7.16, 3.0))
fig.subplots_adjust(left=0.055, right=0.875, bottom=0.27, top=0.90,
                    wspace=0.55)

for ax, (title, chunks) in zip(axes, panels):
    n_rows = len(chunks)
    for row, chunk in chunks:
        # --- attempt events ---
        att = chunk['attempts']
        if att:
            gens = np.array([a[0] for a in att], dtype=float)
            stats = [a[1] for a in att]
            yj = row + rng.uniform(-0.16, 0.16, size=len(att))
            killed = np.array([s == 'killed' for s in stats])
            ok = np.array([s == 'ok' for s in stats])
            err = np.array([s in ERROR_STATUSES for s in stats])
            ax.scatter(gens[killed], yj[killed], marker='|', s=14,
                       c='#949494', alpha=0.5, linewidths=0.6, zorder=2)
            ax.scatter(gens[err], yj[err], marker='x', s=8, c='#D55E00',
                       linewidths=0.6, zorder=3)
            ax.scatter(gens[ok], yj[ok], marker='o', s=10, c='#029E73',
                       edgecolors='none', zorder=4)
        # --- scored proposals (dq == false) ---
        props = [p for p in chunk['proposals'] if not p[2]]
        if props:
            pg = [p[0] for p in props]
            pt = [p[1] for p in props]
            ax.scatter(pg, [row] * len(props), s=26, c=pt, cmap=cmap,
                       norm=norm, edgecolors='black', linewidths=0.3,
                       zorder=5)
        # --- champion score at right edge of row ---
        champ = chunk.get('champion')
        if champ is not None:
            tr = mtransforms.blended_transform_factory(ax.transAxes,
                                                       ax.transData)
            ax.text(1.02, row, f"{champ['total']:.0f}", transform=tr,
                    fontsize=6, fontweight='bold', va='center', ha='left',
                    color='black', clip_on=False)
    ax.set_title(title, pad=4)
    ax.set_xlim(-6, 306)
    ax.set_xticks([1, 100, 200, 300])
    ax.set_xlabel('generation (index)')
    ax.set_ylim(6.6, 0.4)  # chunk 1 at top; shared scale across panels
    ax.set_yticks(range(1, n_rows + 1))
    ax.set_ylabel('chunk (index)')
    ax.tick_params(length=2.5, width=0.7)

# note in the empty lower half of the V7 panel
axes[2].text(150, 5.0, 'first 3 chunks shown', fontsize=6, style='italic',
             color='#949494', ha='center', va='center')

# --- single shared colorbar at far right ---
sm = ScalarMappable(norm=norm, cmap=cmap)
cax = fig.add_axes([0.935, 0.27, 0.013, 0.63])
cb = fig.colorbar(sm, cax=cax)
cb.set_label('proposal score /100', fontsize=7)
cb.ax.tick_params(labelsize=6, length=2, width=0.6)
cb.outline.set_linewidth(0.5)

# --- frameless figure-level legend below panels ---
handles = [
    Line2D([], [], marker='|', linestyle='none', color='#949494', alpha=0.5,
           markersize=5, markeredgewidth=0.8, label='attempt killed by oracle'),
    Line2D([], [], marker='o', linestyle='none', color='#029E73',
           markersize=3, label='attempt ok'),
    Line2D([], [], marker='x', linestyle='none', color='#D55E00',
           markersize=3, markeredgewidth=0.8, label='parse/LLM/other error'),
    Line2D([], [], marker='o', linestyle='none',
           markerfacecolor=cmap(norm(55)), markeredgecolor='black',
           markeredgewidth=0.3, markersize=5, color='none',
           label='scored proposal (color = total)'),
]
fig.legend(handles=handles, loc='lower center', ncol=4, frameon=False,
           bbox_to_anchor=(0.5, 0.0), handletextpad=0.4, columnspacing=1.2)

os.makedirs(os.path.dirname(OUT_PDF), exist_ok=True)
plt.savefig(OUT_PDF, bbox_inches='tight')
plt.savefig(OUT_PNG, dpi=200, bbox_inches='tight')

for p in (OUT_PDF, OUT_PNG):
    sz = os.path.getsize(p)
    assert sz > 0, f'empty file: {p}'
    print(p, sz, 'bytes')
