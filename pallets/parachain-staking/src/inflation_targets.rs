use sp_runtime::Perbill;

pub const YEARLY_INFLATION_TARGET: Perbill = Perbill::from_percent(5);

/// Inflation rate (I) is approximately
///     I = i * x
/// where
///     i = interest rate;
///     x = staking rate;
#[test]
pub(crate) fn derive_inflation_config() {
	let mut results = vec![];
	// Interest rates on staked tokens starting at 10%
	for i in 10..20 {
		// Ideally a minimum of 50% of tokens should be staked
		for x in 50..100 {
			let staking_rate = Perbill::from_percent(x as u32);
			let inflation_rate = staking_rate * Perbill::from_percent(i as u32);
			let diff = YEARLY_INFLATION_TARGET.max(inflation_rate) - YEARLY_INFLATION_TARGET.min(inflation_rate);
			if (diff == Perbill::from_percent(1)) | (diff == Perbill::from_percent(0)) {
				if !results.contains(&(i as u32, x as u32)) {
					results.push((i as u32, x as u32));
				}
			}
		}
	}

	for (i, x) in results {
		println!(
			"\n5% yearly inflation at {}% interest rate and {}% of total tokens staked\n",
			i, x
		);
	}
}
