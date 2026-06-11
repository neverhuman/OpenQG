//! Skeptic's audit: decompose the v6-chunk-4 champion's delta-lnL over the fit set,
//! observable by observable, baseline planck_lcdm vs the champion's BOUND background.
use openqg_core::cosmology::{BackgroundForwardModel, CosmologyParams, ForwardModel};
use openqg_core::scoring::{chi2_quadratic_form, CovarianceBlock};
use openqg_core::types::ObservableRecord;

fn load_obs(path: &str) -> Vec<ObservableRecord> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn load_block(path: &str) -> (Vec<String>, Vec<Vec<f64>>) {
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let ids: Vec<String> = v["observable_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect();
    let matrix: Vec<Vec<f64>> = v["matrix"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row.as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect()
        })
        .collect();
    (ids, matrix)
}

fn main() {
    let obs = load_obs("data/fixtures/cosmology/tier1-multisector.jsonl");
    println!("observables: {}", obs.len());

    // Champion bound background (white-paper genome + binding mu0 = -0.1 from planck_mu0_geff).
    let mut champ = CosmologyParams::planck_lcdm();
    champ.h = 0.7010168574344923;
    champ.omega_m = 0.2935008115235614;
    champ.w0 = -1.0657136202450013;
    champ.mu0 = -0.1;

    // Same background WITHOUT the bound MG (mu0 = 0): isolates what the mechanism contributes.
    let mut champ_nomu = champ.clone();
    champ_nomu.mu0 = 0.0;

    let base = CosmologyParams::planck_lcdm();
    let model = BackgroundForwardModel;
    let ids: Vec<String> = obs.iter().map(|o| o.observable_id.clone()).collect();
    let pb = model.predict(&base, &ids).unwrap();
    let pc = model.predict(&champ, &ids).unwrap();
    let pn = model.predict(&champ_nomu, &ids).unwrap();
    let get = |preds: &[openqg_core::types::PredictionRecord], id: &str| -> f64 {
        preds.iter().find(|p| p.observable_id == id).unwrap().value
    };

    // Covariance blocks: planck 3x3 + 5 desi pairs.
    let planck = load_block("data/fixtures/cosmology/covariance/planck18-distance-priors.json");
    let desi: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string("data/fixtures/cosmology/covariance/desi-dr1-bao.json").unwrap(),
    )
    .unwrap();
    let mut blocks: Vec<CovarianceBlock> = vec![CovarianceBlock {
        ids: planck.0,
        matrix: planck.1,
    }];
    if let Some(arr) = desi["blocks"].as_array() {
        for b in arr {
            let ids: Vec<String> = b["observable_ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().to_string())
                .collect();
            let matrix: Vec<Vec<f64>> = b["matrix"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    row.as_array()
                        .unwrap()
                        .iter()
                        .map(|x| x.as_f64().unwrap())
                        .collect()
                })
                .collect();
            blocks.push(CovarianceBlock { ids, matrix });
        }
    }
    println!("blocks: {}", blocks.len());

    let mut in_block: std::collections::BTreeSet<String> = Default::default();
    let mut tot_b = 0.0; // baseline chi2
    let mut tot_c = 0.0; // champion chi2
    let mut tot_n = 0.0; // champion-no-mu chi2
    println!("\n--- covariance blocks ---");
    for blk in &blocks {
        for id in &blk.ids {
            in_block.insert(id.clone());
        }
        let rb: Vec<f64> = blk
            .ids
            .iter()
            .map(|id| {
                let o = obs.iter().find(|o| &o.observable_id == id).unwrap();
                o.value - get(&pb, id)
            })
            .collect();
        let rc: Vec<f64> = blk
            .ids
            .iter()
            .map(|id| {
                let o = obs.iter().find(|o| &o.observable_id == id).unwrap();
                o.value - get(&pc, id)
            })
            .collect();
        let rn: Vec<f64> = blk
            .ids
            .iter()
            .map(|id| {
                let o = obs.iter().find(|o| &o.observable_id == id).unwrap();
                o.value - get(&pn, id)
            })
            .collect();
        let cb = chi2_quadratic_form(&rb, &blk.matrix).unwrap();
        let cc = chi2_quadratic_form(&rc, &blk.matrix).unwrap();
        let cn = chi2_quadratic_form(&rn, &blk.matrix).unwrap();
        tot_b += cb;
        tot_c += cc;
        tot_n += cn;
        println!(
            "block [{}]: chi2 base={:8.2} champ={:8.2} champ_no_mu={:8.2}  dlnL(block)={:+8.2}",
            blk.ids.join(","),
            cb,
            cc,
            cn,
            0.5 * (cb - cc)
        );
    }
    println!("\n--- diagonal observables ---");
    for o in &obs {
        if in_block.contains(&o.observable_id) {
            continue;
        }
        let vb = get(&pb, &o.observable_id);
        let vc = get(&pc, &o.observable_id);
        let vn = get(&pn, &o.observable_id);
        let s = o.uncertainty;
        let cb = ((o.value - vb) / s).powi(2);
        let cc = ((o.value - vc) / s).powi(2);
        let cn = ((o.value - vn) / s).powi(2);
        tot_b += cb;
        tot_c += cc;
        tot_n += cn;
        println!(
            "{:<18} data={:8.4}  base={:8.4} (pull {:+6.2})  champ={:8.4} (pull {:+6.2})  no_mu={:8.4}  dlnL={:+8.2}",
            o.observable_id, o.value, vb, (vb - o.value) / s, vc, (vc - o.value) / s, vn, 0.5 * (cb - cc)
        );
    }
    println!(
        "\nTOTALS: chi2 base={:.2} champ={:.2} champ_no_mu={:.2}",
        tot_b, tot_c, tot_n
    );
    println!(
        "delta_lnL (champ - base)       = {:+.3}",
        0.5 * (tot_b - tot_c)
    );
    println!(
        "delta_lnL (champ_no_mu - base) = {:+.3}",
        0.5 * (tot_b - tot_n)
    );
    println!(
        "mu0=-0.1 mechanism contribution = {:+.3}",
        0.5 * (tot_n - tot_c)
    );

    // Key per-observable physics numbers.
    println!(
        "\nbaseline lA = {:.4}, champ lA = {:.4} (data 301.471 +/- 0.090)",
        get(&pb, "cmb_lA"),
        get(&pc, "cmb_lA")
    );
    println!(
        "baseline R  = {:.5}, champ R  = {:.5} (data 1.7502 +/- 0.0046)",
        get(&pb, "cmb_R"),
        get(&pc, "cmb_R")
    );
    println!(
        "baseline S8 = {:.4}, champ S8 = {:.4} (data 0.776 +/- 0.017)",
        get(&pb, "s8"),
        get(&pc, "s8")
    );
    println!(
        "champ z* = {:.2}, r_s(z*) = {:.3} Mpc; base z* = {:.2}, r_s(z*) = {:.3} Mpc",
        champ.z_star(),
        champ.sound_horizon(champ.z_star()),
        base.z_star(),
        base.sound_horizon(base.z_star())
    );

    // Counterfactual: calibrate out the engine's lA bias at the Planck best-fit (where the
    // Chen+2019 priors GUARANTEE the true LCDM prediction sits at the measured mean), then
    // rescore the CMB block. If the champion's advantage survives, it is physics; if it flips,
    // it was fitting the engine's r_s/z* fitting-formula bias.
    let bias_la = get(&pb, "cmb_lA") - 301.4707; // engine lA at Planck best-fit minus Planck mean
    let bias_r = get(&pb, "cmb_R") - 1.750235;
    let blk = &blocks[0];
    let data_vals: Vec<f64> = blk
        .ids
        .iter()
        .map(|id| obs.iter().find(|o| &o.observable_id == id).unwrap().value)
        .collect();
    let corr = |preds: &[openqg_core::types::PredictionRecord]| -> Vec<f64> {
        vec![
            data_vals[0] - (get(preds, "cmb_R") - bias_r),
            data_vals[1] - (get(preds, "cmb_lA") - bias_la),
            data_vals[2] - get(preds, "cmb_omega_b_h2"),
        ]
    };
    let cb_corr = chi2_quadratic_form(&corr(&pb), &blk.matrix).unwrap();
    let cc_corr = chi2_quadratic_form(&corr(&pc), &blk.matrix).unwrap();
    println!(
        "\nbias-corrected CMB block (lA bias {:+.4}, R bias {:+.5}):",
        bias_la, bias_r
    );
    println!(
        "  chi2 base={:.2} champ={:.2}  dlnL(block)={:+.2}",
        cb_corr,
        cc_corr,
        0.5 * (cb_corr - cc_corr)
    );
    let tot_b_corr = tot_b
        - chi2_quadratic_form(
            &blk.ids
                .iter()
                .map(|id| {
                    let o = obs.iter().find(|o| &o.observable_id == id).unwrap();
                    o.value - get(&pb, id)
                })
                .collect::<Vec<f64>>(),
            &blk.matrix,
        )
        .unwrap()
        + cb_corr;
    let tot_c_corr = tot_c
        - chi2_quadratic_form(
            &blk.ids
                .iter()
                .map(|id| {
                    let o = obs.iter().find(|o| &o.observable_id == id).unwrap();
                    o.value - get(&pc, id)
                })
                .collect::<Vec<f64>>(),
            &blk.matrix,
        )
        .unwrap()
        + cc_corr;
    println!(
        "  TOTAL delta_lnL with calibrated CMB block: {:+.2} (was {:+.2})",
        0.5 * (tot_b_corr - tot_c_corr),
        0.5 * (tot_b - tot_c)
    );

    // Drag-mechanism sanity: what does Gamma do for phantom w with A_drag > 0?
    let mut dragged = CosmologyParams::planck_lcdm();
    dragged.w0 = -1.066;
    dragged.drag_a = 5.0;
    let fs8_gr: f64 = CosmologyParams::planck_lcdm().growth_fsigma8(0.5);
    let mut wcdm = CosmologyParams::planck_lcdm();
    wcdm.w0 = -1.066;
    println!(
        "\nfs8(0.5): GR/LCDM={:.4}  wCDM(w=-1.066)={:.4}  wCDM+drag_a=5={:.4}  Gamma(a=1)={:+.4}",
        fs8_gr,
        wcdm.growth_fsigma8(0.5),
        dragged.growth_fsigma8(0.5),
        dragged.growth_drag_gamma(1.0)
    );
    let mut dragq = CosmologyParams::planck_lcdm();
    dragq.w0 = -0.9;
    dragq.drag_a = 5.0;
    let mut wq = CosmologyParams::planck_lcdm();
    wq.w0 = -0.9;
    println!(
        "fs8(0.5): wCDM(w=-0.9)={:.4}  wCDM(w=-0.9)+drag_a=5={:.4}  Gamma(a=1)={:+.4}",
        wq.growth_fsigma8(0.5),
        dragq.growth_fsigma8(0.5),
        dragq.growth_drag_gamma(1.0)
    );
}
