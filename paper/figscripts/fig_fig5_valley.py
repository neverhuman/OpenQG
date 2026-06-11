#!/usr/bin/env python3
"""fig5_valley: waterfall/bar resolution of the degeneracy valley."""
import json
import os

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

plt.rcParams.update({
    'font.family': 'DejaVu Sans', 'font.size': 8, 'axes.titlesize': 9,
    'axes.labelsize': 8, 'legend.fontsize': 7, 'xtick.labelsize': 7,
    'ytick.labelsize': 7, 'axes.spines.top': False, 'axes.spines.right': False,
    'axes.linewidth': 0.7, 'figure.dpi': 110,
})

PAL = ['#0173B2', '#DE8F05', '#029E73', '#D55E00', '#CC78BC', '#56B4E9', '#949494', '#ECE133']
BLUE, ORANGE = PAL[0], PAL[1]

with open('/home/ubuntu/openQG/paper/data/story.json') as f:
    story = json.load(f)

valley = story['valley']
vals = [valley['v5_diag'], valley['v6_cov'], valley['v61_diag'], valley['v61_cov']]
labels = ['V5\ndiagonal', 'V6\ncovariance', 'V6.1 diag\n(calibrated\n+Occam)', 'V6.1\ncovariance']

la = story['la_bias']
bias = la['raw'] - la['published']            # +0.7546
zsig = bias / la['sigma']                     # 8.38 sigma
nats = 0.5 * zsig ** 2                        # ~35.1 nats

fig, ax = plt.subplots(figsize=(3.5, 2.7))

x = range(4)
colors = [ORANGE if v > 0 else BLUE for v in vals]
ax.bar(x, vals, width=0.62, color=colors, edgecolor='none', zorder=2)

# zero line
ax.axhline(0, color='black', linewidth=0.7, zorder=3)

# value labels on bars
for xi, v in zip(x, vals):
    if v > 0:
        ax.text(xi, v + 2.5, f'+{v:.1f}', ha='center', va='bottom', fontsize=6)
    else:
        ax.text(xi, v - 2.5, f'{v:.1f}', ha='center', va='top', fontsize=6)

# annotation above the V6 bar: lA bias, thin straight arrow
ax.annotate(
    f'engine $\\ell_A$ bias +{bias:.3f} ({zsig:.1f}$\\sigma$)\n'
    f'~{nats:.0f} nats of model error',
    xy=(1.18, 70.0), xytext=(3.42, 92.0),
    ha='right', va='center', fontsize=6,
    arrowprops=dict(arrowstyle='-|>', lw=0.6, color='0.25',
                    shrinkA=2, shrinkB=1),
)

# annotation at the transition bar 2 -> bar 3: curved arrow
ax.annotate(
    'anchor calibration\n+ Occam $0.5\\,k\\ln n$',
    xy=(1.95, 2.5), xytext=(1.18, 36.0),
    ha='center', va='center', fontsize=6,
    arrowprops=dict(arrowstyle='-|>', lw=0.6, color='0.25',
                    connectionstyle='arc3,rad=-0.35',
                    shrinkA=4, shrinkB=1),
)

ax.set_xticks(list(x))
ax.set_xticklabels(labels, fontsize=7)
ax.set_ylabel(r'$\Delta\ln Z$ of the $h$–$\Omega_m$ drift direction [nats]', fontsize=7)
ax.set_xlim(-0.65, 3.65)
ax.set_ylim(-48, 108)
ax.tick_params(axis='x', length=0)

# bottom note (6 pt)
fig.text(0.5, -0.035,
         'same drift direction ($h$=0.718, $\\Omega_m$=0.282, $w_0$=$-$1.14) '
         'under successive rubrics',
         ha='center', va='top', fontsize=6, color='0.35')

fig.tight_layout()

out_dir = '/home/ubuntu/openQG/paper/figs'
os.makedirs(out_dir, exist_ok=True)
pdf = os.path.join(out_dir, 'fig5_valley.pdf')
png = os.path.join(out_dir, 'fig5_valley.png')
fig.savefig(pdf, bbox_inches='tight')
fig.savefig(png, dpi=200, bbox_inches='tight')

for p in (pdf, png):
    sz = os.path.getsize(p)
    assert sz > 0, f'empty file: {p}'
    print(p, sz, 'bytes')
print(f'bias={bias:.4f}  z={zsig:.2f}sigma  nats={nats:.1f}')
