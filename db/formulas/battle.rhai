// Hit rate formula
fn hit_rate(level, dex, luk, flee) {
    let rate = level + dex + luk / 3 + 175 - flee;
    clamp(rate, 5, 100)
}

// Defense reduction (Renewal)
fn defense_reduction_re(raw, hard_def, soft_def) {
    let d = max(hard_def, -399);
    max(raw * (4000 + d) / (4000 + 10 * d) - soft_def, 0)
}

// Defense reduction (Pre-Renewal)
fn defense_reduction_pre(raw, hard_def, soft_def) {
    max(raw * (100 - min(hard_def, 100)) / 100 - soft_def, 0)
}
