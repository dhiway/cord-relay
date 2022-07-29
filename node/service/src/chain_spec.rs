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

//! CORD chain configurations.

use cord_runtime as cord;
use cord_runtime_constants::currency::WAY;
use grandpa::AuthorityId as GrandpaId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use polkadot_primitives::v2::{AccountId, AccountPublic, AssignmentId, ValidatorId};
use sc_chain_spec::{ChainSpecExtension, ChainType};
use serde::{Deserialize, Serialize};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::IdentifyAccount;
use telemetry::TelemetryEndpoints;

// Note this is the URL for the telemetry server
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.dway.io/submit/";
const DEFAULT_PROTOCOL_ID: &str = "cord";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<polkadot_primitives::v2::Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<polkadot_primitives::v2::Block>,
	/// The light sync state.
	///
	/// This value will be set by the `sync-state rpc` implementation.
	pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// The `ChainSpec` parameterized for the cord runtime.
pub type CordChainSpec = service::GenericChainSpec<cord::GenesisConfig, Extensions>;

pub fn cord_config() -> Result<CordChainSpec, String> {
	CordChainSpec::from_json_bytes(&include_bytes!("../res/cord.json")[..])
}

/// The default parachains host configuration.
fn default_parachains_host_configuration(
) -> polkadot_runtime_parachains::configuration::HostConfiguration<
	polkadot_primitives::v2::BlockNumber,
> {
	use polkadot_primitives::v2::{MAX_CODE_SIZE, MAX_POV_SIZE};

	polkadot_runtime_parachains::configuration::HostConfiguration {
		validation_upgrade_cooldown: 2u32,
		validation_upgrade_delay: 2,
		code_retention_period: 1200,
		max_code_size: MAX_CODE_SIZE,
		max_pov_size: MAX_POV_SIZE,
		max_head_data_size: 32 * 1024,
		group_rotation_frequency: 20,
		chain_availability_period: 4,
		thread_availability_period: 4,
		max_upward_queue_count: 8,
		max_upward_queue_size: 1024 * 1024,
		max_downward_message_size: 1024 * 1024,
		ump_service_total_weight: 100_000_000_000,
		max_upward_message_size: 50 * 1024,
		max_upward_message_num_per_candidate: 5,
		hrmp_sender_deposit: 0,
		hrmp_recipient_deposit: 0,
		hrmp_channel_max_capacity: 8,
		hrmp_channel_max_total_size: 8 * 1024,
		hrmp_max_parachain_inbound_channels: 4,
		hrmp_max_parathread_inbound_channels: 4,
		hrmp_channel_max_message_size: 1024 * 1024,
		hrmp_max_parachain_outbound_channels: 4,
		hrmp_max_parathread_outbound_channels: 4,
		hrmp_max_message_num_per_candidate: 5,
		dispute_period: 6,
		no_show_slots: 2,
		n_delay_tranches: 25,
		needed_approvals: 2,
		relay_vrf_modulo_samples: 2,
		zeroth_delay_tranche_width: 0,
		minimum_validation_upgrade_delay: 5,
		..Default::default()
	}
}

#[test]
fn default_parachains_host_configuration_is_consistent() {
	default_parachains_host_configuration().panic_if_not_consistent();
}

fn session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	im_online: ImOnlineId,
	para_validator: ValidatorId,
	para_assignment: AssignmentId,
	authority_discovery: AuthorityDiscoveryId,
) -> cord::SessionKeys {
	cord::SessionKeys {
		babe,
		grandpa,
		im_online,
		para_validator,
		para_assignment,
		authority_discovery,
	}
}

/// Returns the properties for the [`PolkadotChainSpec`].
pub fn cord_chain_spec_properties() -> serde_json::map::Map<String, serde_json::Value> {
	serde_json::json!({
		"tokenSymbol": "WAY",
		"tokenDecimals": 12,
		"ss58Format": 29,
	})
	.as_object()
	.expect("Map given; qed")
	.clone()
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(
	seed: &str,
) -> (
	AccountId,
	AccountId,
	BabeId,
	GrandpaId,
	ImOnlineId,
	ValidatorId,
	AssignmentId,
	AuthorityDiscoveryId,
) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<ValidatorId>(seed),
		get_from_seed::<AssignmentId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

fn testnet_accounts() -> Vec<AccountId> {
	vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Charlie"),
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie"),
		get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
		get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
	]
}

/// Helper function to create polkadot `GenesisConfig` for testing
pub fn cord_testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ImOnlineId,
		ValidatorId,
		AssignmentId,
		AuthorityDiscoveryId,
	)>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> cord::GenesisConfig {
	let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);

	const ENDOWMENT: u128 = 10_000 * WAY;
	const STASH: u128 = 100 * WAY;

	cord::GenesisConfig {
		system: cord::SystemConfig { code: wasm_binary.to_vec() },
		indices: cord::IndicesConfig { indices: vec![] },
		balances: cord::BalancesConfig {
			balances: endowed_accounts.iter().map(|k| (k.clone(), ENDOWMENT)).collect(),
		},
		session: cord::SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						session_keys(
							x.2.clone(),
							x.3.clone(),
							x.4.clone(),
							x.5.clone(),
							x.6.clone(),
							x.7.clone(),
						),
					)
				})
				.collect::<Vec<_>>(),
		},
		phragmen_election: cord::PhragmenElectionConfig {
			members: endowed_accounts
				.iter()
				.take((endowed_accounts.len() + 1) / 2)
				.cloned()
				.map(|member| (member, STASH))
				.collect(),
		},
		democracy: cord::DemocracyConfig::default(),
		council: cord::CouncilConfig { members: vec![], phantom: Default::default() },
		technical_committee: cord::TechnicalCommitteeConfig {
			members: vec![],
			phantom: Default::default(),
		},
		technical_membership: Default::default(),
		babe: cord::BabeConfig {
			authorities: Default::default(),
			epoch_config: Some(cord::BABE_GENESIS_EPOCH_CONFIG),
		},
		grandpa: Default::default(),
		im_online: Default::default(),
		authority_discovery: cord::AuthorityDiscoveryConfig { keys: vec![] },
		sudo: cord::SudoConfig { key: Some(root_key) },
		treasury: Default::default(),
		hrmp: Default::default(),
		configuration: cord::ConfigurationConfig {
			config: default_parachains_host_configuration(),
		},
		paras: Default::default(),
		xcm_pallet: Default::default(),
	}
}

fn cord_development_config_genesis(wasm_binary: &[u8]) -> cord::GenesisConfig {
	cord_testnet_genesis(
		wasm_binary,
		vec![get_authority_keys_from_seed("Alice")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Cord development config (single validator Alice)
pub fn cord_development_config() -> Result<CordChainSpec, String> {
	let wasm_binary = cord::WASM_BINARY.ok_or("CORD development wasm not available")?;

	Ok(CordChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		move || cord_development_config_genesis(wasm_binary),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Some(cord_chain_spec_properties()),
		Default::default(),
	))
}

fn cord_local_testnet_genesis(wasm_binary: &[u8]) -> cord::GenesisConfig {
	cord_testnet_genesis(
		wasm_binary,
		vec![get_authority_keys_from_seed("Alice"), get_authority_keys_from_seed("Bob")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// CORD local testnet config (multivalidator Alice + Bob)
pub fn cord_local_testnet_config() -> Result<CordChainSpec, String> {
	let wasm_binary = cord::WASM_BINARY.ok_or("CORD development wasm not available")?;

	Ok(CordChainSpec::from_genesis(
		"Cord Local Testnet",
		"cord_local_testnet",
		ChainType::Local,
		move || cord_local_testnet_genesis(wasm_binary),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Some(cord_chain_spec_properties()),
		Default::default(),
	))
}

fn cord_staging_testnet_config_genesis(wasm_binary: &[u8]) -> cord::GenesisConfig {
	use hex_literal::hex;
	use sp_core::crypto::UncheckedInto;
	let endowed_accounts: Vec<AccountId> = vec![
		//3wsLrSkvbtkHWJMV9HfgsBZUvb7Tj2ZcmpbjZwaKJBBDp2Nt
		hex!["8664c9618257c3fbbc190212de4c3d11c25200205b5116b014442047a58bbe2c"].into(),
		//3yGbbRbWvwyuAstobpg2T8cE952SUsWEVVmRdqP1iqYJDea2
		hex!["c45d49820d82543d26eea9dec6d993a977e094699ecf55bc6258c8466c09ea02"].into(),
		//3uS6AYUHSvXxqFJDHxgPLDFcEJkXm3s4YypT3MHaMHwoAQLr
		hex!["1aa9bd954312d77d6670bbf931d3b2b6ea0808194f72c607e4f15d7cf2301900"].into(),
	];

	let initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ImOnlineId,
		ValidatorId,
		AssignmentId,
		AuthorityDiscoveryId,
	)> = vec![
		(
			//3xL7dsND1fMez1y8jnhX6UHKKu8H59oGPi3Ei6isBzYCCyft
			hex!["9acfebb79e08fa28e0d9b11f5a3bf52892e7bad2c181995b677eabefbc937053"].into(),
			//3wsLrSkvbtkHWJMV9HfgsBZUvb7Tj2ZcmpbjZwaKJBBDp2Nt
			hex!["8664c9618257c3fbbc190212de4c3d11c25200205b5116b014442047a58bbe2c"].into(),
			//3vo1jm48k9Erh6Wy2WGEmBSkwTwSZVKpzqZaL89thGRcH9Xw
			hex!["56db3951c3fe6a33251f4ac1722890ce7bf8d291cc3096326bbc88451eee8561"]
				.unchecked_into(),
			//3x8A5hP7LZJ3rt3zhPne53v6k4GiykjKQxBfw8rbGFtLMgwL
			hex!["91b1352287a57568a84a45f3d61294775066e5b121d57d2dd591389ce9f775e6"]
				.unchecked_into(),
			//3uJBGFvs9uTVtgSBCrzn7oESCRwNUjxd4zHr3S7HyLuJXFjV
			hex!["14a0f0c5363c402ce94b59987fa4a4c4e5d2d3bf8eb256032771dd322b555c36"]
				.unchecked_into(),
			//3yYkrigAkYPkewAhkSy4mwAxqRPraqEfsia4hYxEzgG1Wdth
			hex!["d0b065e330a99c1411c2efd49b1419a9130cc14495560fc175cc05d054f00f79"]
				.unchecked_into(),
			//3xYC27DVgCjCRPDX6KshgvUAVqPJiv34a6eLHSjhnvtGZ8FJ
			hex!["a4059f1e382fb365d468f49ba70321f6c322944e14b9693e36d145529eaf0163"]
				.unchecked_into(),
			//3vka9h1XLvM49cXnwncQXiAr7pWyt4cfiVxBrrb3NjnEfAwc
			hex!["54fe9923739db23103829382436ce176c53d3e21db1a86b1a56c4adbe83a7f47"]
				.unchecked_into(),
		),
		(
			//3vCdm8HKtBSD9xRoCCjNspBQw4VYLL5xM9Vb7HyHusiYHwRp
			hex!["3ca2e69d28c837c7b8ac477a75d513ebd58facf8a1572e86a1df87652d9c521f"].into(),
			//3yGbbRbWvwyuAstobpg2T8cE952SUsWEVVmRdqP1iqYJDea2
			hex!["c45d49820d82543d26eea9dec6d993a977e094699ecf55bc6258c8466c09ea02"].into(),
			//3usUmujYHcUZBywcmWqKcjx2Jnp15fZYSjNNxjGQ2NFyKDnJ
			hex!["2e06fca7c688f518db8307d63a75771712e6de558b4ea47875f818cde6e7f33b"]
				.unchecked_into(),
			//3ugJgwMS3gYn9UR8r6NoNB4k3nPZmpBxELVTT5pEVr6iPVPJ
			hex!["258155fe10bb1fd9fee2fc15107867db3c2e38f1fbf0974536643a03f9878d5b"]
				.unchecked_into(),
			//3yNTRuTVLdXg62xm367wBjCmxVj8fsYLT5iXKWtn2MWvzwnx
			hex!["c8d546d28ea2bc420d41a9edc75f8c8f6b7e522943f9c351aafbebbbf937273b"]
				.unchecked_into(),
			//3uPgW6KDnqufN865hoQcPRy1PxPE1pFH5F1f1KdaSMEhm1hc
			hex!["18d397d9cf5594e890de24c608a926aeb56b674e23bb691e226abeb962a05a26"]
				.unchecked_into(),
			//3yNDKjZsjsEhqaAHYEs7Di2Eze39YiaQZqSmhiASgpxYPp8b
			hex!["c8a5ca5210723b0fd6e9fec4da349fab6fa254b10785de86f255b9709390ca32"]
				.unchecked_into(),
			//3xseW7QhunG9NTCB3VPpZrPfCHvVFxyxCUgmyjFbZqte2Xf9
			hex!["b2dc6eef996e11b0b7ef930776da6860ad894033d2000f67ed21570463ef7814"]
				.unchecked_into(),
		),
		(
			//3vYt8yDez8sDtYnnA4uTDsHhX7pxQ6KXEZPPkrQ86biBipQJ
			hex!["4c143476154b3fab8a217c2e5bfde25384344c4345731b38ee0996b32205f774"].into(),
			//3uS6AYUHSvXxqFJDHxgPLDFcEJkXm3s4YypT3MHaMHwoAQLr
			hex!["1aa9bd954312d77d6670bbf931d3b2b6ea0808194f72c607e4f15d7cf2301900"].into(),
			//3xTPvTCPASoZi7cgPMKaYZh5jFUSoQpUSm8r7jWp8C9RBPC6
			hex!["a05d763e74b7364b855cf17b5f0f3bf64058f3af047463fd8367dcc6982e7d56"]
				.unchecked_into(),
			//3xTsSs6qjXvVZStr3Qesgt9UZmSVjuengLEynFTNx5GNFcK4
			hex!["a0ba1dfa76ac1f59c6597ad8b6a763523bd4c007a6939574b885ce07289a3c7b"]
				.unchecked_into(),
			//3xBJVVybkteXwQrze1Tu856neDEivCSZjkXdPZ4JYSp17SuV
			hex!["941741763d2a43df112d6d557e41d2a64e63875d184e05261942e613c8dbdd0b"]
				.unchecked_into(),
			//3yMfqAuRDdyXN1fukm79pbTigBiVouCrDPRQdjbc18jk8Yr8
			hex!["c83bc6f7aac502db862144af4248ac146216dcd63808faafcd51db35a38fee50"]
				.unchecked_into(),
			//3yDgX8wZ4enJcZ9Vyiqc9uPX43EdtXbrrEw2JxdepqcdQGmo
			hex!["c2242233d5b9152bfb5f039eee108d145a7afd3b0275bd13869ea9844c91bb60"]
				.unchecked_into(),
			//3xYcS4DMzKUCMnr7KccfR4m91KJmtGmyKJLs7ngYSTBqC6rn
			hex!["a457cd7e03d0341633ea335901bf88663a6eea543744bcd39fcc1d9ed1ca7160"]
				.unchecked_into(),
		),
	];
	let root_key: AccountId = endowed_accounts[0].clone();

	const ENDOWMENT: u128 = 1_000_000 * WAY;
	const STASH: u128 = 100 * WAY;

	cord::GenesisConfig {
		system: cord::SystemConfig { code: wasm_binary.to_vec() },
		balances: cord::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.map(|k: &AccountId| (k.clone(), ENDOWMENT))
				.chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
				.collect(),
		},
		indices: cord::IndicesConfig { indices: vec![] },
		session: cord::SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						session_keys(
							x.2.clone(),
							x.3.clone(),
							x.4.clone(),
							x.5.clone(),
							x.6.clone(),
							x.7.clone(),
						),
					)
				})
				.collect::<Vec<_>>(),
		},
		phragmen_election: cord::PhragmenElectionConfig {
			members: endowed_accounts
				.iter()
				.take((endowed_accounts.len() + 1) / 2)
				.cloned()
				.map(|member| (member, STASH))
				.collect(),
		},
		democracy: Default::default(),
		council: cord::CouncilConfig { members: vec![], phantom: Default::default() },
		technical_committee: cord::TechnicalCommitteeConfig {
			members: vec![],
			phantom: Default::default(),
		},
		technical_membership: Default::default(),
		babe: cord::BabeConfig {
			authorities: Default::default(),
			epoch_config: Some(cord::BABE_GENESIS_EPOCH_CONFIG),
		},
		grandpa: Default::default(),
		im_online: Default::default(),
		authority_discovery: cord::AuthorityDiscoveryConfig { keys: vec![] },
		sudo: cord::SudoConfig { key: Some(root_key) },
		treasury: Default::default(),
		hrmp: Default::default(),
		configuration: cord::ConfigurationConfig {
			config: default_parachains_host_configuration(),
		},
		paras: Default::default(),
		xcm_pallet: Default::default(),
	}
}

/// CORD staging testnet config.
pub fn cord_staging_testnet_config() -> Result<CordChainSpec, String> {
	let wasm_binary = cord::WASM_BINARY.ok_or("CORD development wasm not available")?;
	let boot_nodes = vec![];

	Ok(CordChainSpec::from_genesis(
		"CORD Staging Testnet",
		"cord_staging_testnet",
		ChainType::Live,
		move || cord_staging_testnet_config_genesis(wasm_binary),
		boot_nodes,
		Some(
			TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
				.expect("CORD Staging telemetry url is valid; qed"),
		),
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Some(cord_chain_spec_properties()),
		Default::default(),
	))
}
