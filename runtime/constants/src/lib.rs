// Copyright (C) 2019-2022 Dhiway Networks Pvt. Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// This file is part of CORD - `https://cord.network` relay node
// based on Polkadot & Substrate framework."

// CORD is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// CORD is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with CORD. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;

/// Money matters.
pub mod currency {
	use primitives::v2::Balance;

	/// The existential deposit.
	pub const EXISTENTIAL_DEPOSIT: Balance = 1 * MICRO_WAY;

	pub const WAY: Balance = 10u128.pow(12);
	pub const UNITS: Balance = WAY;
	pub const MILLI_WAY: Balance = 10u128.pow(9); // mWAY
	pub const MICRO_WAY: Balance = 10u128.pow(6); // uWAY
	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 200 * MILLI_WAY + (bytes as Balance) * 100 * MICRO_WAY
	}
}

/// Time and blocks.
pub mod time {
	use primitives::v2::{BlockNumber, Moment};
	use runtime_common::prod_or_fast;
	pub const MILLISECS_PER_BLOCK: Moment = 6000;
	pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;
	pub const EPOCH_DURATION_IN_SLOTS: BlockNumber = prod_or_fast!(2 * MINUTES, 1 * MINUTES);

	// These time units are defined in number of blocks.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;
	pub const WEEKS: BlockNumber = DAYS * 7;

	// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
	pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);
}

/// Fee-related.
pub mod fee {
	use crate::weights::ExtrinsicBaseWeight;
	use frame_support::weights::{
		WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
	};
	use primitives::v2::Balance;
	use smallvec::smallvec;
	pub use sp_runtime::Perbill;

	/// The block saturation level. Fees will be updates based on this value.
	pub const TARGET_BLOCK_FULLNESS: Perbill = Perbill::from_percent(25);

	/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
	/// node's balance type.
	///
	/// This should typically create a mapping between the following ranges:
	///   - [0, `MAXIMUM_BLOCK_WEIGHT`]
	///   - [Balance::min, Balance::max]
	///
	/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
	///   - Setting it to `0` will essentially disable the weight fee.
	///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
	pub struct WeightToFee;
	impl WeightToFeePolynomial for WeightToFee {
		type Balance = Balance;
		fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
			// in Kusama, extrinsic base weight (smallest non-zero weight) is mapped to 1/10 CENT:
			let p = super::currency::MICRO_WAY;
			let q = 10 * Balance::from(ExtrinsicBaseWeight::get());
			smallvec![WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::from_rational(p % q, q),
				coeff_integer: p / q,
			}]
		}
	}
}

// #[cfg(test)]
// mod tests {
// 	use super::{
// 		currency::{CENTS, MILLICENTS},
// 		fee::WeightToFee,
// 	};
// 	use crate::weights::ExtrinsicBaseWeight;
// 	use frame_support::weights::WeightToFee as WeightToFeeT;
// 	use runtime_common::MAXIMUM_BLOCK_WEIGHT;

// 	#[test]
// 	// Test that the fee for `MAXIMUM_BLOCK_WEIGHT` of weight has sane bounds.
// 	fn full_block_fee_is_correct() {
// 		// A full block should cost between 1,000 and 10,000 CENTS.
// 		let full_block = WeightToFee::weight_to_fee(&MAXIMUM_BLOCK_WEIGHT);
// 		assert!(full_block >= 1_000 * CENTS);
// 		assert!(full_block <= 10_000 * CENTS);
// 	}

// 	#[test]
// 	// This function tests that the fee for `ExtrinsicBaseWeight` of weight is correct
// 	fn extrinsic_base_fee_is_correct() {
// 		// `ExtrinsicBaseWeight` should cost 1/10 of a CENT
// 		println!("Base: {}", ExtrinsicBaseWeight::get());
// 		let x = WeightToFee::weight_to_fee(&ExtrinsicBaseWeight::get());
// 		let y = CENTS / 10;
// 		assert!(x.max(y) - x.min(y) < MILLICENTS);
// 	}
// }
