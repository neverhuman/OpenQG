#!/usr/bin/env python3
"""fig6_data: admitted dataset pulls (left) + growth-sector zoom (right)."""
import json
import os
import re

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib import gridspec

plt.rcParams.update({
    'font.family': 'DejaVu Sans', 'font.size': 8, 'axes.titlesize': 9,
    'axes.labelsize': 8, 'legend.fontsize': 7, 'xtick.labelsize': 7,
    'ytick.labelsize': 7, 'axes.spines.top': False,
    'axes.spines.right': False, 'axes.linewidth': 0.7, 'figure.dpi': 110,
})

C_LCDM, C_MU0, C_DRAG, C_GRAY = '#0173B2', '#029E73', '#DE8F05', '#949494'

DATA = '/home/ubuntu/openQG/paper/data'
OUT_PDF = '/home/ubuntu/openQG/paper/figs/fig6_data.pdf'
OUT_PNG = '/home/ubuntu/openQG/paper/figs/fig6_data.png'

# ---------------- load ----------------
obs = {}
with open(os.path.join(DATA, 'observables.jsonl')) as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        d = json.loads(line)
        obs[d['id']] = (float(d['value']), float(d['unc']))

pred = {'lcdm': {}, 'mu0': {}, 'drag': {}}
with open(os.path.join(DATA, 'predictions.txt')) as f:
    for line in f:
        parts = line.split()
        if len(parts) == 4 and parts[0] == 'PROBE':
            _, model, oid, val = parts
            pred[model][oid] = float(val)

common = [oid for oid in obs if all(oid in pred[m] for m in pred)]

# ---------------- ordering per spec ----------------
def zof(oid):
    m = re.search(r'@([\d.]+)', oid)
    return float(m.group(1)) if m else None

fs8_ids = sorted([o for o in common if o.startswith('fsigma8@')], key=zof)
bao_rank = {'dv': 0, 'dm': 1, 'dh': 2}
bao_ids = sorted([o for o in common if '_over_rd@' in o],
                 key=lambda o: (zof(o), bao_rank[o[:2]]))
order = (['h0', 's8'] + fs8_ids + bao_ids
         + ['cmb_R', 'cmb_lA', 'cmb_omega_b_h2'])
order = [o for o in order if o in common]
assert len(order) == len(common), (len(order), len(common))

def label(oid):
    if oid == 'h0':
        return r'$H_0$'
    if oid == 's8':
        return r'$S_8$'
    if oid == 'cmb_R':
        return r'CMB $R$'
    if oid == 'cmb_lA':
        return r'CMB $\ell_A$'
    if oid == 'cmb_omega_b_h2':
        return r'CMB $\omega_b$'
    z = zof(oid)
    if oid.startswith('fsigma8'):
        return r'$f\sigma_8({:g})$'.format(z)
    kind = {'dv': 'D_V', 'dm': 'D_M', 'dh': 'D_H'}[oid[:2]]
    return r'${}/r_d({:g})$'.format(kind, z)

def pulls(model):
    return [(obs[o][0] - pred[model][o]) / obs[o][1] for o in order]

p_lcdm, p_mu0, p_drag = pulls('lcdm'), pulls('mu0'), pulls('drag')
for name, p in (('lcdm', p_lcdm), ('mu0', p_mu0), ('drag', p_drag)):
    print(name, ['%s:%.2f' % (o, v) for o, v in zip(order, p)])

# ---------------- figure ----------------
fig = plt.figure(figsize=(7.16, 3.2))
gs = gridspec.GridSpec(1, 2, width_ratios=[5, 2], wspace=0.30,
                       left=0.07, right=0.985, top=0.96, bottom=0.30)
axL = fig.add_subplot(gs[0])
axR = fig.add_subplot(gs[1])

# ===== LEFT: pulls =====
x = list(range(len(order)))
YLO, YHI = -4.3, 6.9
CLIP = 6.3  # off-scale points pinned here

axL.axhspan(-1, 1, color=C_GRAY, alpha=0.08, lw=0, zorder=0)
axL.axhline(0, color=C_GRAY, lw=0.5, alpha=0.6, zorder=1)
for yy in (-2, 2):
    axL.axhline(yy, color=C_GRAY, lw=0.5, ls='--', alpha=0.8, zorder=1)

dx = 0.16  # horizontal dodge so coincident pulls stay visible
axL.plot([xi - dx for xi in x], p_lcdm, '-', color=C_LCDM, lw=0.7,
         marker='o', ms=3.2, mew=0, zorder=4, label=r'Planck $\Lambda$CDM')
axL.plot(x, p_mu0, ls='none', marker='s', ms=3.4, mfc='none', mec=C_MU0,
         mew=0.9, zorder=5, label=r'$\mu_0=-0.1$ (certified)')
p_drag_clip = [min(v, CLIP) for v in p_drag]
axL.plot([xi + dx for xi in x], p_drag_clip, ls='none', marker='^', ms=3.8,
         mfc='none', mec=C_DRAG, mew=0.9, zorder=5,
         label=r'dark scattering ($A=2$, $w_0=-0.9$)')

# annotate off-scale drag points
for xi, v in zip(x, p_drag):
    if v > CLIP:
        axL.annotate(r'$+%.1f\sigma$' % v, xy=(xi + dx, CLIP),
                     xytext=(xi + dx - 0.25, CLIP - 1.25),
                     ha='right', va='top', fontsize=6, color=C_DRAG,
                     arrowprops=dict(arrowstyle='->', lw=0.5, color=C_DRAG,
                                     shrinkA=1, shrinkB=2))

# annotate H0 tension (lcdm point at x=0-dx)
i_h0 = order.index('h0')
axL.annotate(r'$H_0$ tension $+5.4\sigma$',
             xy=(i_h0 - dx, p_lcdm[i_h0]), xytext=(1.6, 6.1),
             fontsize=7, ha='left', va='center',
             arrowprops=dict(arrowstyle='->', lw=0.5, color='0.25',
                             shrinkA=2, shrinkB=3))

axL.set_xticks(x)
axL.set_xticklabels([label(o) for o in order], rotation=90, fontsize=7)
axL.set_xlim(-0.8, len(order) - 0.2)
axL.set_ylim(YLO, YHI)
axL.set_xlabel('Observable')
axL.set_ylabel(r'Pull  $(x_{\rm obs}-x_{\rm model})/\sigma$  [$\sigma$]')
axL.legend(loc='upper center', bbox_to_anchor=(0.56, 1.02), frameon=False,
           handlelength=1.6, borderaxespad=0.0)

# ===== RIGHT: growth sector =====
zs = [zof(o) for o in fs8_ids]
d_val = [obs[o][0] for o in fs8_ids]
d_unc = [obs[o][1] for o in fs8_ids]

def model_fs8(model):
    return [pred[model][o] for o in fs8_ids]

axR.errorbar(zs, d_val, yerr=d_unc, fmt='o', color='0.15', ms=3.0,
             elinewidth=0.7, capsize=1.8, capthick=0.7, zorder=6,
             label='data')
axR.plot(zs, model_fs8('lcdm'), '-', color=C_LCDM, lw=0.9, marker='o',
         ms=2.8, mew=0, zorder=4)
axR.plot(zs, model_fs8('mu0'), '-', color=C_MU0, lw=0.9, marker='s',
         ms=2.8, mfc='none', mew=0.8, zorder=4)
axR.plot(zs, model_fs8('drag'), '-', color=C_DRAG, lw=0.9, marker='^',
         ms=3.0, mfc='none', mew=0.8, zorder=4)

axR.set_xlabel(r'Redshift $z$')
axR.set_ylabel(r'$f\sigma_8(z)$')
axR.set_xlim(-0.08, 1.62)
axR.set_ylim(0.34, 0.56)
axR.legend(loc='upper right', frameon=False, handlelength=1.2,
           borderaxespad=0.0)

fig.savefig(OUT_PDF, bbox_inches='tight')
fig.savefig(OUT_PNG, dpi=200, bbox_inches='tight')

for p in (OUT_PDF, OUT_PNG):
    sz = os.path.getsize(p)
    assert sz > 0, p
    print(os.path.abspath(p), sz, 'bytes')
