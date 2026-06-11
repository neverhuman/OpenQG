#!/usr/bin/env python3
"""fig2_timeline: ZYAL V4->V7 horizontal timeline with cascade-of-champions overlay."""
import json, os

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 9,
                     'axes.titlesize': 9, 'axes.labelsize': 8, 'legend.fontsize': 7,
                     'xtick.labelsize': 7, 'ytick.labelsize': 7,
                     'axes.spines.top': False, 'axes.spines.right': False,
                     'axes.linewidth': 0.7, 'figure.dpi': 110})

PAL = ['#0173B2', '#DE8F05', '#029E73', '#D55E00', '#CC78BC', '#56B4E9', '#949494', '#ECE133']

DATA = '/home/ubuntu/openQG/paper/data/story.json'
OUT_PDF = '/home/ubuntu/openQG/paper/figs/fig2_timeline.pdf'
OUT_PNG = '/home/ubuntu/openQG/paper/figs/fig2_timeline.png'

story = json.load(open(DATA))
timeline = story['timeline']          # 10 events
cascade = story['cascade']            # 5 eras

assert len(timeline) == 10, f"expected 10 events, got {len(timeline)}"

# --- map each cascade era to the timeline slot where its champion appeared ---
era_slot = {
    'V4': 0,              # "V4 trust spine; 87.5 (gamed)"
    'V5': 3,              # "V5 campaign 6x300"
    'V6': 6,              # "V6 campaign: 77.0 champion"
    'V6.1 survivor': 7,   # "21-agent audit; V6.1"
    'V7': 9,              # "V7 campaign: honest 43-class champions"
}
casc_x = [era_slot[c['era']] for c in cascade]
casc_y = [c['champion'] for c in cascade]

# short labels above stems (derived from timeline labels)
short = ['V4 spine', 'V4.1 audit', 'V5 binding', 'V5 campaign', 'V6 plan',
         'V6 purge', 'V6 campaign', 'V6.1 audit', 'V7 design', 'V7 campaign']

fig, ax = plt.subplots(figsize=(7.16, 2.9))
ax2 = ax.twinx()

# ---------------- left axis: timeline geometry (0..1 internal units) -------
ax.set_xlim(-0.6, 9.6)
ax.set_ylim(0, 1)
ax.set_yticks([])
ax.spines['left'].set_visible(False)

SPINE_Y = 0.10
STEM = [0.10, 0.17]  # alternating stem heights above the spine

# live-campaign background spans (V5, V6, V7 campaign slots)
span_cols = [PAL[0], PAL[2], PAL[4]]
for (slot, col) in zip([3, 6, 9], span_cols):
    ax.axvspan(slot - 0.45, slot + 0.45, color=col, alpha=0.06, lw=0, zorder=0)
    # keep the slot-9 label inside the axes (its box is wider than the span)
    ax.text(min(slot, 8.85), 0.985, 'live campaign\n6×300 gens', ha='center',
            va='top', fontsize=7.5, color=col, zorder=5)

# central horizontal spine
ax.plot([-0.45, 9.45], [SPINE_Y, SPINE_Y], color='#444444', lw=1.0,
        solid_capstyle='round', zorder=2)

# events: stems + markers + short labels, t values as minor text under spine
for i, ev in enumerate(timeline):
    top = SPINE_Y + STEM[i % 2]
    ax.plot([i, i], [SPINE_Y, top], color=PAL[6], lw=0.7, zorder=2)
    ax.plot(i, top, marker='o', ms=3.5, mfc=PAL[0], mec=PAL[0], zorder=3)
    ax.text(i, top + 0.025, short[i], ha='center', va='bottom', fontsize=7,
            color='#222222', zorder=4)
    ax.text(i, SPINE_Y - 0.045, ev['t'], ha='center', va='top', fontsize=8.0,
            color=PAL[6], zorder=4)

# full event labels under the axis, rotated 28 deg
ax.set_xticks(range(10))
ax.set_xticklabels([ev['label'] for ev in timeline], rotation=28,
                   ha='right', rotation_mode='anchor', fontsize=7)
ax.set_xlabel('event (chronological, June 2026; t = MM-DD)', labelpad=2)
ax.tick_params(axis='x', length=2, pad=2)

# ---------------- right axis: champion-score cascade ------------------------
# ylim extends below 0 so the score band floats above the stem labels;
# the visible right spine/ticks still read 0-100.
ax2.set_ylim(-55, 122)
ax2.set_yticks([0, 25, 50, 75, 100])
ax2.spines['right'].set_visible(True)
ax2.spines['right'].set_bounds(0, 100)
ax2.spines['right'].set_color(PAL[3])
ax2.spines['top'].set_visible(False)
ax2.tick_params(axis='y', colors=PAL[3], length=2)
ax2.set_ylabel('champion score (rubric points, 0–100)', color=PAL[3])

ax2.plot(casc_x, casc_y, color=PAL[3], lw=1.0, zorder=3)
ax2.plot(casc_x[:-1], casc_y[:-1], 'o', ms=3.5, mfc=PAL[3], mec=PAL[3], zorder=4)
# final standing champion: star
ax2.plot(casc_x[-1], casc_y[-1], marker='*', ms=10, mfc=PAL[3], mec='#7a3100',
         mew=0.5, ls='none', zorder=5)

# score value next to the first four cascade points (final 43 is carried
# by the "standing" annotation next to the star)
score_off = [(0.0, 4, 'center', 'bottom'), (-0.15, -5, 'right', 'center'),
             (0.22, -3, 'left', 'center'), (-0.16, 0, 'right', 'center')]
for (x, y, (dx, dy, ha, va)) in zip(casc_x[:4], casc_y[:4], score_off):
    ax2.text(x + dx, y + dy, f'{y:g}', ha=ha, va=va, fontsize=7.5,
             color=PAL[3], fontweight='bold', zorder=5)

# "killed by ..." annotations, 6 pt italic, next to each death
ax2.text(0.62, 98, 'killed by V4.1\n(rubric-gaming audit)', fontsize=7,
         style='italic', color='#333333', ha='left', va='top', zorder=5)
ax2.text(3.14, 51, 'killed by V6\n(adjudication + drift cost)', fontsize=7,
         style='italic', color='#333333', ha='left', va='top', zorder=5)
ax2.text(6.0, 81, 'killed by V6.1\n(ℓA bias + novelty laundering)', fontsize=7,
         style='italic', color='#333333', ha='center', va='bottom', zorder=5)
ax2.text(7.18, 14, 'killed by V7\n(term registry)', fontsize=7,
         style='italic', color='#333333', ha='left', va='center', zorder=5)
ax2.text(8.82, 52, 'honest 43 — standing', fontsize=7, style='italic',
         color='#333333', ha='right', va='center', zorder=5)

ax.set_title('ZYAL V4→V7: each champion score is killed by the next audit',
             pad=4)

fig.savefig(OUT_PDF, bbox_inches='tight')
fig.savefig(OUT_PNG, dpi=200, bbox_inches='tight')

for p in (OUT_PDF, OUT_PNG):
    sz = os.path.getsize(p)
    assert sz > 0, p
    print(p, sz, 'bytes')
