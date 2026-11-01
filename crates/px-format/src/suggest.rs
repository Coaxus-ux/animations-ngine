const MAX_NAME: usize = 64;

pub(crate) fn closest<'a>(
    name: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let limit = (name.chars().count() / 3).max(1);
    candidates
        .into_iter()
        .map(|candidate| (distance(name, candidate), candidate))
        .filter(|(distance, _)| *distance <= limit)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}

pub(crate) fn did_you_mean<'a>(
    name: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> String {
    closest(name, candidates)
        .map(|found| format!(" Did you mean `{found}`?"))
        .unwrap_or_default()
}

pub(crate) fn hint<'a>(
    name: &str,
    candidates: impl IntoIterator<Item = &'a str> + Clone,
) -> String {
    closest(name, candidates.clone()).map_or_else(
        || {
            format!(
                " Known: {}.",
                candidates.into_iter().collect::<Vec<_>>().join(", ")
            )
        },
        |found| format!(" Did you mean `{found}`?"),
    )
}

fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (
        a.chars().take(MAX_NAME).collect(),
        b.chars().take(MAX_NAME).collect(),
    );
    let mut rows = vec![vec![0; b.len() + 1]; a.len() + 1];
    for (i, row) in rows.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in rows[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (rows[i - 1][j] + 1)
                .min(rows[i][j - 1] + 1)
                .min(rows[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(rows[i - 2][j - 2] + 1);
            }
            rows[i][j] = best;
        }
    }
    rows[a.len()][b.len()]
}
