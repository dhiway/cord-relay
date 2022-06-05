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

//! XCM configurations for the CORD runtime.

use super::{
	parachains_origin, AccountId, Balances, Call, CouncilCollective, Event, Origin, ParaId,
	Runtime, WeightToFee, XcmPallet,
};
use frame_support::{
	parameter_types,
	traits::{Everything, IsInVec, Nothing},
	weights::Weight,
};
use runtime_common::{xcm_sender, ToAuthor};
use sp_std::prelude::*;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom, BackingToPlurality,
	ChildParachainAsNative, ChildParachainConvertsVia, ChildSystemParachainAsSuperuser,
	CurrencyAdapter as XcmCurrencyAdapter, FixedWeightBounds, IsConcrete, LocationInverter,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, UsingComponents,
};

parameter_types! {
	pub const WayLocation: MultiLocation = Here.into();
	pub const RelayNetwork: NetworkId = NetworkId::Any;
	pub const Ancestry: MultiLocation = Here.into();
	pub CheckAccount: AccountId = XcmPallet::check_account();
}

/// The canonical means of converting a `MultiLocation` into an `AccountId`, used when we want to determine
/// the sovereign account controlled by a location.
pub type SovereignAccountOf = (
	// We can convert a child parachain using the standard `AccountId` conversion.
	ChildParachainConvertsVia<ParaId, AccountId>,
	// We can directly alias an `AccountId32` into a local account.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

/// Our asset transactor. This is what allows us to interest with the runtime facilities from the point of
/// view of XCM-only concepts like `MultiLocation` and `MultiAsset`.
///
/// Ours is only aware of the Balances pallet, which is mapped to `KsmLocation`.
pub type LocalAssetTransactor = XcmCurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<WayLocation>,
	// We can convert the MultiLocations with our converter above:
	SovereignAccountOf,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We track our teleports in/out to keep total issuance correct.
	CheckAccount,
>;

/// The means that we convert an the XCM message origin location into a local dispatch origin.
type LocalOriginConverter = (
	// A `Signed` origin of the sovereign account that the original location controls.
	SovereignSignedViaLocation<SovereignAccountOf, Origin>,
	// A child parachain, natively expressed, has the `Parachain` origin.
	ChildParachainAsNative<parachains_origin::Origin, Origin>,
	// The AccountId32 location type can be expressed natively as a `Signed` origin.
	SignedAccountId32AsNative<RelayNetwork, Origin>,
	// A system child parachain, expressed as a Superuser, converts to the `Root` origin.
	ChildSystemParachainAsSuperuser<ParaId, Origin>,
);

parameter_types! {
	/// The amount of weight an XCM operation takes. This is a safe overestimate.
	pub const BaseXcmWeight: Weight = 1_000_000_000;
	/// Maximum number of instructions in a single XCM fragment. A sanity check against weight
	/// calculations getting too crazy.
	pub const MaxInstructions: u32 = 100;
}

/// The XCM router. When we want to send an XCM message, we use this type. It amalgamates all of our
/// individual routers.
pub type XcmRouter = (
	// Only one router so far - use DMP to communicate with child parachains.
	xcm_sender::ChildParachainRouter<Runtime, XcmPallet>,
);

parameter_types! {
	pub const Cord: MultiAssetFilter = Wild(AllOf { fun: WildFungible, id: Concrete(WayLocation::get()) });
	pub const CordForStreams: (MultiAssetFilter, MultiLocation) = (Cord::get(), Parachain(100).into());
	pub const CordForAssets: (MultiAssetFilter, MultiLocation) = (Cord::get(), Parachain(110).into());
}
pub type TrustedTeleporters = (xcm_builder::Case<CordForStreams>, xcm_builder::Case<CordForAssets>);

parameter_types! {
	pub AllowUnpaidFrom: Vec<MultiLocation> =
		vec![
			Parachain(100).into(),
			Parachain(110).into(),
		];
}

use xcm_builder::{AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, TakeWeightCredit};
/// The barriers one of which must be passed for an XCM message to be executed.
pub type Barrier = (
	// Weight that is paid for may be consumed.
	TakeWeightCredit,
	// If the message is one that immediately attemps to pay for execution, then allow it.
	AllowTopLevelPaidExecutionFrom<Everything>,
	// Messages coming from system parachains need not pay for execution.
	AllowUnpaidExecutionFrom<IsInVec<AllowUnpaidFrom>>,
	// Expected responses are OK.
	AllowKnownQueryResponses<XcmPallet>,
	// Subscriptions for version tracking are OK.
	AllowSubscriptionsFrom<Everything>,
);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = LocalOriginConverter;
	type IsReserve = ();
	type IsTeleporter = TrustedTeleporters;
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<BaseXcmWeight, Call, MaxInstructions>;
	// The weight trader piggybacks on the existing transaction-fee conversion logic.
	type Trader = UsingComponents<WeightToFee, WayLocation, AccountId, Balances, ToAuthor<Runtime>>;
	type ResponseHandler = XcmPallet;
	type AssetTrap = XcmPallet;
	type AssetClaims = XcmPallet;
	type SubscriptionService = XcmPallet;
}

parameter_types! {
	pub const CouncilBodyId: BodyId = BodyId::Executive;
}

/// Type to convert an `Origin` type value into a `MultiLocation` value which represents an interior location
/// of this chain.
pub type LocalOriginToLocation = (
	// We allow an origin from the Collective pallet to be used in XCM as a corresponding Plurality of the
	// `Unit` body.
	BackingToPlurality<
		Origin,
		pallet_collective::Origin<Runtime, CouncilCollective>,
		CouncilBodyId,
	>,
	// And a usual Signed origin to be used in XCM as a corresponding AccountId32
	SignedToAccountId32<Origin, AccountId, RelayNetwork>,
);
impl pallet_xcm::Config for Runtime {
	type Event = Event;
	// We don't allow any messages to be sent via the transaction yet. This is basically safe to
	// enable, (safe the possibility of someone spamming the parachain if they're willing to pay
	// the DOT to send from the Relay-chain). But it's useless until we bring in XCM v3 which will
	// make `DescendOrigin` a bit more useful.
	type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	// Anyone can execute XCM messages locally.
	type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmExecuteFilter = Nothing;
	type XcmExecutor = xcm_executor::XcmExecutor<XcmConfig>;
	// Anyone is able to use teleportation regardless of who they are and what they want to teleport.
	type XcmTeleportFilter = Everything;
	// Anyone is able to use reserve transfers regardless of who they are and what they want to
	// transfer.
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<BaseXcmWeight, Call, MaxInstructions>;
	type LocationInverter = LocationInverter<Ancestry>;
	type Origin = Origin;
	type Call = Call;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}
