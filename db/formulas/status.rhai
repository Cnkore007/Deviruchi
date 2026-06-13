// Stat point cost (Renewal)
fn stat_point_cost_re(current) {
    if current < 100 { 2 + (current - 1) / 10 }
    else { 16 + 4 * ((current - 100) / 5) }
}

// Stat point cost (Pre-Renewal)
fn stat_point_cost_pre(current) {
    (current + 9) / 10 + 1
}
