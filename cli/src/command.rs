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

use crate::cli::{Cli, Subcommand};
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use futures::future::TryFutureExt;
use sc_cli::{Role, RuntimeVersion, SubstrateCli};
use service::{self, HeaderBackend, IdentifyVariant};
use std::net::ToSocketAddrs;

pub use crate::{error::Error, service::BlockId};

impl From<String> for Error {
	fn from(s: String) -> Self {
		Self::Other(s)
	}
}

type Result<T> = std::result::Result<T, Error>;

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Dhiway CORD".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/dhiway/cord/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn executable_name() -> String {
		"cord".into()
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		let spec = match id {
			"" => {
				return Err(
					"Please specify which chain you want to run, e.g. --dev or --chain=local"
						.into(),
				)
			},
			"cord" => Box::new(service::chain_spec::cord_config()?),
			"cord-dev" | "dev" => Box::new(service::chain_spec::cord_development_config()?),
			"cord-local" | "local" => Box::new(service::chain_spec::cord_local_testnet_config()?),
			// "cord-staging" | "staging" => Box::new(service::chain_spec::cord_staging_testnet_config()?),
			path => {
				Box::new(service::CordChainSpec::from_json_file(std::path::PathBuf::from(path))?)
			},
		};
		Ok(spec)
	}

	fn native_runtime_version(_spec: &Box<dyn service::ChainSpec>) -> &'static RuntimeVersion {
		&service::cord_runtime::VERSION
	}
}

const DEV_ONLY_ERROR_PATTERN: &'static str =
	"can only use subcommand with --chain [cord-dev, dev], got ";

fn ensure_dev(spec: &Box<dyn service::ChainSpec>) -> std::result::Result<(), String> {
	if spec.is_dev() {
		Ok(())
	} else {
		Err(format!("{}{}", DEV_ONLY_ERROR_PATTERN, spec.id()))
	}
}

/// Unwraps a [`cord_client::Client`] into the concrete runtime client.
macro_rules! unwrap_client {
	(
		$client:ident,
		$code:expr
	) => {
		match $client.as_ref() {
			#[cfg(feature = "cord-native")]
			cord_client::Client::Cord($client) => $code,
			#[allow(unreachable_patterns)]
			_ => Err(Error::CommandNotImplemented),
		}
	};
}

/// Runs performance checks.
/// Should only be used in release build since the check would take too much time otherwise.
fn host_perf_check() -> Result<()> {
	#[cfg(not(build_type = "release"))]
	{
		Err(Error::WrongBuildType)
	}
	#[cfg(build_type = "release")]
	{
		crate::host_perf_check::host_perf_check()?;
		Ok(())
	}
}

/// Launch a node, accepting arguments just like a regular node,
/// accepts an alternative overseer generator, to adjust behavior
/// for integration tests as needed.
#[cfg(feature = "malus")]
pub fn run_node(run: Cli, overseer_gen: impl service::OverseerGen) -> Result<()> {
	run_node_inner(run, overseer_gen, |_logger_builder, _config| {})
}

fn run_node_inner<F>(
	cli: Cli,
	overseer_gen: impl service::OverseerGen,
	logger_hook: F,
) -> Result<()>
where
	F: FnOnce(&mut sc_cli::LoggerBuilder, &sc_service::Configuration),
{
	let runner = cli
		.create_runner_with_logger_hook::<sc_cli::RunCmd, F>(&cli.run.base, logger_hook)
		.map_err(Error::from)?;
	let chain_spec = &runner.config().chain_spec;

	// Disallow BEEFY on production networks.
	if cli.run.beefy && (chain_spec.is_cord()) {
		return Err(Error::Other("BEEFY disallowed on production networks".to_string()));
	}

	let grandpa_pause = if cli.run.grandpa_pause.is_empty() {
		None
	} else {
		Some((cli.run.grandpa_pause[0], cli.run.grandpa_pause[1]))
	};

	let jaeger_agent = if let Some(ref jaeger_agent) = cli.run.jaeger_agent {
		Some(
			jaeger_agent
				.to_socket_addrs()
				.map_err(Error::AddressResolutionFailure)?
				.next()
				.ok_or_else(|| Error::AddressResolutionMissing)?,
		)
	} else {
		None
	};

	runner.run_node_until_exit(move |config| async move {
		let hwbench = if !cli.run.no_hardware_benchmarks {
			config.database.path().map(|database_path| {
				let _ = std::fs::create_dir_all(&database_path);
				sc_sysinfo::gather_hwbench(Some(database_path))
			})
		} else {
			None
		};

		let role = config.role.clone();

		match role {
			Role::Light => Err(Error::Other("Light client not enabled".into())),
			_ => service::build_full(
				config,
				service::IsCollator::No,
				grandpa_pause,
				cli.run.beefy,
				jaeger_agent,
				None,
				false,
				overseer_gen,
				cli.run.overseer_channel_capacity_override,
				hwbench,
			)
			.map(|full| full.task_manager)
			.map_err(Into::into),
		}
	})
}

/// Parses polkadot specific CLI arguments and run the service.
pub fn run() -> Result<()> {
	let cli: Cli = Cli::from_args();

	#[cfg(feature = "pyroscope")]
	let mut pyroscope_agent_maybe = if let Some(ref agent_addr) = cli.run.pyroscope_server {
		let address = agent_addr
			.to_socket_addrs()
			.map_err(Error::AddressResolutionFailure)?
			.next()
			.ok_or_else(|| Error::AddressResolutionMissing)?;
		// The pyroscope agent requires a `http://` prefix, so we just do that.
		let mut agent = pyro::PyroscopeAgent::builder(
			"http://".to_owned() + address.to_string().as_str(),
			"polkadot".to_owned(),
		)
		.sample_rate(113)
		.build()?;
		agent.start();
		Some(agent)
	} else {
		None
	};

	#[cfg(not(feature = "pyroscope"))]
	if cli.run.pyroscope_server.is_some() {
		return Err(Error::PyroscopeNotCompiledIn);
	}

	match &cli.subcommand {
		None => run_node_inner(cli, service::RealOverseerGen, polkadot_node_metrics::logger_hook()),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			Ok(runner.sync_run(|config| cmd.run(config.chain_spec, config.network))?)
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd).map_err(Error::SubstrateCli)?;

			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) =
					service::new_chain_ops(&mut config, None)?;
				Ok((cmd.run(client, import_queue).map_err(Error::SubstrateCli), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			Ok(runner.async_run(|mut config| {
				let (client, _, _, task_manager) =
					service::new_chain_ops(&mut config, None).map_err(Error::CordService)?;
				Ok((cmd.run(client, config.database).map_err(Error::SubstrateCli), task_manager))
			})?)
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			Ok(runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops(&mut config, None)?;
				Ok((cmd.run(client, config.chain_spec).map_err(Error::SubstrateCli), task_manager))
			})?)
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			Ok(runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) =
					service::new_chain_ops(&mut config, None)?;
				Ok((cmd.run(client, import_queue).map_err(Error::SubstrateCli), task_manager))
			})?)
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			Ok(runner.sync_run(|config| cmd.run(config.database))?)
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			Ok(runner.async_run(|mut config| {
				let (client, backend, _, task_manager) = service::new_chain_ops(&mut config, None)?;
				let aux_revert = Box::new(|client, backend, blocks| {
					service::revert_backend(client, backend, blocks, config).map_err(|err| {
						match err {
							service::Error::Blockchain(err) => err.into(),
							// Generic application-specific error.
							err => sc_cli::Error::Application(err.into()),
						}
					})
				});
				Ok((
					cmd.run(client, backend, Some(aux_revert)).map_err(Error::SubstrateCli),
					task_manager,
				))
			})?)
		},
		Some(Subcommand::PvfPrepareWorker(cmd)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_colors(false);
			let _ = builder.init();

			#[cfg(target_os = "android")]
			{
				return Err(sc_cli::Error::Input(
					"PVF preparation workers are not supported under this platform".into(),
				)
				.into());
			}

			#[cfg(not(target_os = "android"))]
			{
				polkadot_node_core_pvf::prepare_worker_entrypoint(&cmd.socket_path);
				Ok(())
			}
		},
		Some(Subcommand::PvfExecuteWorker(cmd)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_colors(false);
			let _ = builder.init();

			#[cfg(target_os = "android")]
			{
				return Err(sc_cli::Error::Input(
					"PVF execution workers are not supported under this platform".into(),
				)
				.into());
			}

			#[cfg(not(target_os = "android"))]
			{
				polkadot_node_core_pvf::execute_worker_entrypoint(&cmd.socket_path);
				Ok(())
			}
		},
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			match cmd {
				BenchmarkCmd::Storage(cmd) => runner.sync_run(|mut config| {
					let (client, backend, _, _) = service::new_chain_ops(&mut config, None)?;
					let db = backend.expose_db();
					let storage = backend.expose_storage();

					unwrap_client!(
						client,
						cmd.run(config, client.clone(), db, storage).map_err(Error::SubstrateCli)
					)
				}),
				BenchmarkCmd::Block(cmd) => runner.sync_run(|mut config| {
					let (client, _, _, _) = service::new_chain_ops(&mut config, None)?;

					unwrap_client!(client, cmd.run(client.clone()).map_err(Error::SubstrateCli))
				}),
				BenchmarkCmd::Overhead(cmd) => {
					ensure_dev(chain_spec).map_err(Error::Other)?;
					runner.sync_run(|mut config| {
						use cord_client::benchmark_inherent_data;
						let (client, _, _, _) = service::new_chain_ops(&mut config, None)?;
						let wrapped = client.clone();

						let header = client.header(BlockId::Number(0_u32.into())).unwrap().unwrap();
						let inherent_data = benchmark_inherent_data(header)
							.map_err(|e| format!("generating inherent data: {:?}", e))?;

						unwrap_client!(
							client,
							cmd.run(config, client.clone(), inherent_data, wrapped)
								.map_err(Error::SubstrateCli)
						)
					})
				},
				BenchmarkCmd::Pallet(cmd) => {
					ensure_dev(chain_spec).map_err(Error::Other)?;

					#[cfg(feature = "cord-native")]
					{
						return Ok(runner.sync_run(|config| {
							cmd.run::<service::cord_runtime::Block, service::CordExecutorDispatch>(
								config,
							)
							.map_err(|e| Error::SubstrateCli(e))
						})?);
					}

					#[cfg(not(feature = "cord-native"))]
					#[allow(unreachable_code)]
					Err(service::Error::NoRuntime.into())
				},
				BenchmarkCmd::Machine(cmd) => runner.sync_run(|config| {
					cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())
						.map_err(Error::SubstrateCli)
				}),
				// NOTE: this allows the Cord client to leniently implement
				// new benchmark commands.
				#[allow(unreachable_patterns)]
				_ => Err(Error::CommandNotImplemented),
			}
		},
		Some(Subcommand::HostPerfCheck) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_colors(true);
			builder.init()?;

			host_perf_check()
		},
		Some(Subcommand::Key(cmd)) => Ok(cmd.run(&cli)?),
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			use sc_service::TaskManager;
			let registry = &runner.config().prometheus_config.as_ref().map(|cfg| &cfg.registry);
			let task_manager = TaskManager::new(runner.config().tokio_handle.clone(), *registry)
				.map_err(|e| Error::SubstrateService(sc_service::Error::Prometheus(e)))?;

			ensure_dev(chain_spec).map_err(Error::Other)?;

			#[cfg(feature = "cord-native")]
			{
				return runner.async_run(|config| {
					Ok((
						cmd.run::<service::cord_runtime::Block, service::CordExecutorDispatch>(
							config,
						)
						.map_err(Error::SubstrateCli),
						task_manager,
					))
				});
			}
			#[cfg(not(feature = "cord-native"))]
			panic!("No runtime feature is enabled")
		},
		#[cfg(not(feature = "try-runtime"))]
		Some(Subcommand::TryRuntime) => Err(Error::Other(
			"TryRuntime wasn't enabled when building the node. \
				You can enable it with `--features try-runtime`."
				.into(),
		)
		.into()),
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			Ok(runner.sync_run(|config| cmd.run::<service::Block>(&config))?)
		},
	}?;

	#[cfg(feature = "pyroscope")]
	if let Some(mut pyroscope_agent) = pyroscope_agent_maybe.take() {
		pyroscope_agent.stop();
	}
	Ok(())
}
