// Copyright 2022 Dhiway Networks Pvt. Ltd.
// This file is part of CORD - `https://cord.network`.
// A relay node implementation based on Polkadot & Substrate.

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

//! CORD-specific GRANDPA integration utilities.

use std::sync::Arc;

use sp_runtime::traits::{Block as BlockT, Header as _, NumberFor};

use crate::HeaderProvider;

#[cfg(feature = "full-node")]

/// Returns the block hash of the block at the given `target_number` by walking
/// backwards from the given `current_header`.
pub(super) fn walk_backwards_to_target_block<Block, HP>(
	backend: &HP,
	target_number: NumberFor<Block>,
	current_header: &Block::Header,
) -> Result<(Block::Hash, NumberFor<Block>), sp_blockchain::Error>
where
	Block: BlockT,
	HP: HeaderProvider<Block>,
{
	let mut target_hash = current_header.hash();
	let mut target_header = current_header.clone();

	loop {
		if *target_header.number() < target_number {
			unreachable!(
				"we are traversing backwards from a known block; \
				 blocks are stored contiguously; \
				 qed"
			);
		}

		if *target_header.number() == target_number {
			return Ok((target_hash, target_number));
		}

		target_hash = *target_header.parent_hash();
		target_header = backend
			.header(target_hash)?
			.expect("Header known to exist due to the existence of one of its descendants; qed");
	}
}

/// A custom GRANDPA voting rule that "pauses" voting (i.e. keeps voting for the
/// same last finalized block) after a given block at height `N` has been
/// finalized and for a delay of `M` blocks, i.e. until the best block reaches
/// `N` + `M`, the voter will keep voting for block `N`.
#[derive(Clone)]
pub(crate) struct PauseAfterBlockFor<N>(pub(crate) N, pub(crate) N);

impl<Block, B> grandpa::VotingRule<Block, B> for PauseAfterBlockFor<NumberFor<Block>>
where
	Block: BlockT,
	B: sp_blockchain::HeaderBackend<Block> + 'static,
{
	fn restrict_vote(
		&self,
		backend: Arc<B>,
		base: &Block::Header,
		best_target: &Block::Header,
		current_target: &Block::Header,
	) -> grandpa::VotingRuleResult<Block> {
		let aux = || {
			// only restrict votes targeting a block higher than the block
			// we've set for the pause
			if *current_target.number() > self.0 {
				// if we're past the pause period (i.e. `self.0 + self.1`)
				// then we no longer need to restrict any votes
				if *best_target.number() > self.0 + self.1 {
					return None;
				}

				// if we've finalized the pause block, just keep returning it
				// until best number increases enough to pass the condition above
				if *base.number() >= self.0 {
					return Some((base.hash(), *base.number()));
				}

				// otherwise find the target header at the pause block
				// to vote on
				return walk_backwards_to_target_block(&*backend, self.0, current_target).ok();
			}

			None
		};

		let target = aux();

		Box::pin(async move { target })
	}
}

#[cfg(test)]
mod tests {
	use consensus_common::BlockOrigin;
	use grandpa::VotingRule;
	use polkadot_test_client::{
		ClientBlockImportExt, DefaultTestClientBuilderExt, InitPolkadotBlockBuilder,
		TestClientBuilder, TestClientBuilderExt,
	};
	use sp_blockchain::HeaderBackend;
	use sp_runtime::{generic::BlockId, traits::Header};
	use std::sync::Arc;

	#[test]
	fn grandpa_pause_voting_rule_works() {
		let _ = env_logger::try_init();

		let client = Arc::new(TestClientBuilder::new().build());

		let mut push_blocks = {
			let mut client = client.clone();

			move |n| {
				for _ in 0..n {
					let block = client.init_polkadot_block_builder().build().unwrap().block;
					futures::executor::block_on(client.import(BlockOrigin::Own, block)).unwrap();
				}
			}
		};

		let get_header = {
			let client = client.clone();
			move |n| client.header(&BlockId::Number(n)).unwrap().unwrap()
		};

		// the rule should filter all votes after block #20
		// is finalized until block #50 is imported.
		let voting_rule = super::PauseAfterBlockFor(20, 30);

		// add 10 blocks
		push_blocks(10);
		assert_eq!(client.info().best_number, 10);

		// we have not reached the pause block
		// therefore nothing should be restricted
		assert_eq!(
			futures::executor::block_on(voting_rule.restrict_vote(
				client.clone(),
				&get_header(0),
				&get_header(10),
				&get_header(10)
			)),
			None,
		);

		// add 15 more blocks
		// best block: #25
		push_blocks(15);

		// we are targeting the pause block,
		// the vote should not be restricted
		assert_eq!(
			futures::executor::block_on(voting_rule.restrict_vote(
				client.clone(),
				&get_header(10),
				&get_header(20),
				&get_header(20)
			)),
			None,
		);

		// we are past the pause block, votes should
		// be limited to the pause block.
		let pause_block = get_header(20);
		assert_eq!(
			futures::executor::block_on(voting_rule.restrict_vote(
				client.clone(),
				&get_header(10),
				&get_header(21),
				&get_header(21)
			)),
			Some((pause_block.hash(), *pause_block.number())),
		);

		// we've finalized the pause block, so we'll keep
		// restricting our votes to it.
		assert_eq!(
			futures::executor::block_on(voting_rule.restrict_vote(
				client.clone(),
				&pause_block, // #20
				&get_header(21),
				&get_header(21),
			)),
			Some((pause_block.hash(), *pause_block.number())),
		);

		// add 30 more blocks
		// best block: #55
		push_blocks(30);

		// we're at the last block of the pause, this block
		// should still be considered in the pause period
		assert_eq!(
			futures::executor::block_on(voting_rule.restrict_vote(
				client.clone(),
				&pause_block, // #20
				&get_header(50),
				&get_header(50),
			)),
			Some((pause_block.hash(), *pause_block.number())),
		);

		// we're past the pause period, no votes should be filtered
		assert_eq!(
			futures::executor::block_on(voting_rule.restrict_vote(
				client.clone(),
				&pause_block, // #20
				&get_header(51),
				&get_header(51),
			)),
			None,
		);
	}
}
