#!/usr/bin/env python3
"""fig3_funnels: outcome funnels per campaign + per-lane OK yields (V6 vs V7)."""
import json
import os
from collections import Counter

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

plt.rcParams.update({
    'font.family': 'DejaVu Sans', 'font.size': 8, 'axes.titlesize': 9,
    'axes.labelsize': 8, 'legend.fontsize': 7, 'xtick.labelsize': 7,
    'ytick.labelsize': 7, 'axes.spines.top': False, 'axes.spines.right': False,
    'axes.linewidth': 0.7, 'figure.dpi': 110,
})

PAL = ['#0173B2', '#DE8F05', '#029E73', '#D55E00', '#CC78BC', '#56B4E9',
       '#949494', '#ECE133']

DATA = '/home/ubuntu/openQG/paper/data'
OUT_PDF = '/home/ubuntu/openQG/paper/figs/fig3_funnels.pdf'
OUT_PNG = '/home/ubuntu/openQG/paper/figs/fig3_funnels.png'

funnels = json.load(open(os.path.join(DATA, 'funnels.json')))
campaigns = json.load(open(os.path.join(DATA, 'campaigns.json')))

# ---------------- top panel data ----------------
seg_order = ['ok', 'killed', 'parse_error', 'llm_error', 'empty_output',
             'engine_error']
seg_color = dict(zip(seg_order, PAL[:6]))
camp_keys = [('v5', 'V5'), ('v6', 'V6'),
             ('v7_partial', 'V7 (chunks 1-3 + partial)')]

# ---------------- bottom panel data ----------------
lanes = ['planck_mu0', 'dark_scattering', 'free', 'null_diagnostic']
ok_per_lane = {}
for camp in ('v6', 'v7'):
    cnt = Counter()
    for chunk in campaigns[camp].values():
        for gen, outcome, lane, model in chunk.get('attempts', []):
            if outcome == 'ok':
                cnt[lane] += 1
    ok_per_lane[camp] = [cnt.get(l, 0) for l in lanes]

# sanity check against funnel totals
assert sum(ok_per_lane['v6']) == funnels['v6']['ok'], 'v6 ok mismatch'
assert sum(ok_per_lane['v7']) == funnels['v7_partial']['ok'], 'v7 ok mismatch'

# ---------------- figure ----------------
fig, (ax1, ax2) = plt.subplots(
    2, 1, figsize=(3.5, 3.6),
    gridspec_kw={'height_ratios': [1.0, 1.05], 'hspace': 1.05})

# --- TOP: horizontal stacked funnel bars ---
ys = [2, 1, 0]  # V5 on top
for (key, label), y in zip(camp_keys, ys):
    f = funnels[key]
    left = 0.0
    for seg in seg_order:
        v = f.get(seg, 0)
        if v:
            ax1.barh(y, v, left=left, height=0.62, color=seg_color[seg],
                     edgecolor='none', label=None)
        left += v
    total = left
    ok = f.get('ok', 0)
    ax1.text(total + 14, y, f'n={int(total)}\nok={ok}', va='center',
             ha='left', fontsize=6, linespacing=1.1)

ax1.set_yticks(ys)
ax1.set_yticklabels(['V5', 'V6', 'V7 (chunks 1-3\n+ partial)'], fontsize=6.5)
ax1.set_xlim(0, 1010)
ax1.set_ylim(-0.55, 3.15)
ax1.set_xlabel('LLM calls (count)')
ax1.set_title('Outcome funnel per campaign', pad=2)

# story annotation: empty_output plague in V5 vanishes afterwards
v5 = funnels['v5']
eo_start = sum(v5.get(s, 0) for s in seg_order[:seg_order.index('empty_output')])
eo_mid = eo_start + v5['empty_output'] / 2.0
ax1.annotate('empty_output (213)\ngone in V6/V7', xy=(eo_mid, 2.34),
             xytext=(430, 2.95), fontsize=6, ha='left', va='center',
             linespacing=1.1,
             arrowprops=dict(arrowstyle='->', lw=0.6, color='#333333',
                             shrinkA=2, shrinkB=1, relpos=(0.0, 0.0)))

# legend (frameless, below top panel)
handles = [plt.Rectangle((0, 0), 1, 1, fc=seg_color[s], ec='none')
           for s in seg_order]
ax1.legend(handles, seg_order, loc='upper center',
           bbox_to_anchor=(0.5, -0.42), ncol=3, frameon=False,
           handlelength=1.0, handleheight=0.8, columnspacing=0.9,
           labelspacing=0.3, borderaxespad=0.0, fontsize=6.5)

# --- BOTTOM: grouped per-lane OK yields ---
x = range(len(lanes))
w = 0.34
b6 = ax2.bar([i - w / 2 for i in x], ok_per_lane['v6'], width=w,
             color='#0173B2', edgecolor='none', label='V6')
b7 = ax2.bar([i + w / 2 for i in x], ok_per_lane['v7'], width=w,
             color='#DE8F05', edgecolor='none', label='V7')
for bars in (b6, b7):
    for r in bars:
        ax2.text(r.get_x() + r.get_width() / 2, r.get_height() + 0.5,
                 f'{int(r.get_height())}', ha='center', va='bottom',
                 fontsize=6)
ax2.set_xticks(list(x))
ax2.set_xticklabels([l.replace('_', '\n') for l in lanes], fontsize=6.5,
                    linespacing=1.1)
ax2.set_ylim(0, 38)
ax2.set_ylabel('OK attempts (count)')
ax2.set_title('OK yield per lane, V6 vs V7', pad=2)
ax2.legend(frameon=False, loc='upper right', fontsize=6.5,
           handlelength=1.0, borderaxespad=0.2)

os.makedirs(os.path.dirname(OUT_PDF), exist_ok=True)
fig.savefig(OUT_PDF, bbox_inches='tight')
fig.savefig(OUT_PNG, dpi=200, bbox_inches='tight')

for p in (OUT_PDF, OUT_PNG):
    sz = os.path.getsize(p)
    assert sz > 0, p
    print(p, sz, 'bytes')
