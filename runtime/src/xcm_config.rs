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

//! XCM configurations for the CORD runtime.

use super::{
	parachains_origin, AccountId, Balances, CouncilCollective, ParaId, Runtime, RuntimeCall,
	RuntimeEvent, RuntimeOrigin, WeightToFee, XcmPallet,
};
use frame_support::{
	match_types, parameter_types,
	traits::{Everything, Nothing},
};
use runtime_common::{xcm_sender, ToAuthor};
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, BackingToPlurality,
	ChildParachainAsNative, ChildParachainConvertsVia, ChildSystemParachainAsSuperuser,
	CurrencyAdapter as XcmCurrencyAdapter, FixedWeightBounds, IsChildSystemParachain, IsConcrete,
	LocationInverter, SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
	TakeWeightCredit, UsingComponents, WeightInfoBounds,
};

parameter_types! {
	/// The location of the WAY token, from the context of this chain. Since this token is native to this
	/// chain, we make it synonymous with it and thus it is the `Here` location, which means "equivalent to
	/// the context".
	pub const WayLocation: MultiLocation = Here.into();
	/// The Cord network ID.
	pub CordNetwork: NetworkId = NetworkId::Named(b"Cord".to_vec().try_into().expect("shorter than length limit; qed"));

	/// Our XCM location ancestry - i.e. what, if anything, `Parent` means evaluated in our context. Since
	/// Cord is a top-level relay-chain, there is no ancestry.
	pub const Ancestry: MultiLocation = Here.into();
	/// The check account, which holds any native assets that have been teleported out and not back in (yet).
	pub CheckAccount: AccountId = XcmPallet::check_account();
}

/// The canonical means of converting a `MultiLocation` into an `AccountId`, used when we want to determine
/// the sovereign account controlled by a location.
pub type SovereignAccountOf = (
	// We can convert a child parachain using the standard `AccountId` conversion.
	ChildParachainConvertsVia<ParaId, AccountId>,
	// We can directly alias an `AccountId32` into a local account.
	AccountId32Aliases<CordNetwork, AccountId>,
);

/// Our asset transactor. This is what allows us to interest with the runtime facilities from the point of
/// view of XCM-only concepts like `MultiLocation` and `MultiAsset`.
///
/// Ours is only aware of the Balances pallet, which is mapped to `WayLocation`.
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
	SovereignSignedViaLocation<SovereignAccountOf, RuntimeOrigin>,
	// A child parachain, natively expressed, has the `Parachain` origin.
	ChildParachainAsNative<parachains_origin::Origin, RuntimeOrigin>,
	// The AccountId32 location type can be expressed natively as a `Signed` origin.
	SignedAccountId32AsNative<CordNetwork, RuntimeOrigin>,
	// A system child parachain, expressed as a Superuser, converts to the `Root` origin.
	ChildSystemParachainAsSuperuser<ParaId, RuntimeOrigin>,
);

parameter_types! {
	/// The amount of weight an XCM operation takes. This is a safe overestimate.
	pub const BaseXcmWeight: u64 = 1_000_000_000;
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
	pub const Stream: MultiLocation = Parachain(1000).into();
	pub const Unique: MultiLocation = Parachain(1001).into();
	pub const CordForStream: (MultiAssetFilter, MultiLocation) = (Cord::get(), Stream::get());
	pub const CordForUnique: (MultiAssetFilter, MultiLocation) = (Cord::get(), Stream::get());
}

pub type TrustedTeleporters = (xcm_builder::Case<CordForStream>, xcm_builder::Case<CordForUnique>);

match_types! {
	pub type OnlyParachains: impl Contains<MultiLocation> = {
		MultiLocation { parents: 0, interior: X1(Parachain(_)) }
	};
}

/// The barriers one of which must be passed for an XCM message to be executed.
pub type Barrier = (
	// Weight that is paid for may be consumed.
	TakeWeightCredit,
	// If the message is one that immediately attemps to pay for execution, then allow it.
	AllowTopLevelPaidExecutionFrom<Everything>,
	// Messages coming from system parachains need not pay for execution.
	AllowUnpaidExecutionFrom<IsChildSystemParachain<ParaId>>,
	// Expected responses are OK.
	AllowKnownQueryResponses<XcmPallet>,
	// Subscriptions for version tracking are OK.
	AllowSubscriptionsFrom<OnlyParachains>,
);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = LocalOriginConverter;
	type IsReserve = ();
	type IsTeleporter = TrustedTeleporters;
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::CordXcmWeight<RuntimeCall>,
		RuntimeCall,
		MaxInstructions,
	>; // The weight trader piggybacks on the existing transaction-fee conversion logic.
	type Trader = UsingComponents<WeightToFee, WayLocation, AccountId, Balances, ToAuthor<Runtime>>;
	type ResponseHandler = XcmPallet;
	type AssetTrap = XcmPallet;
	type AssetClaims = XcmPallet;
	type SubscriptionService = XcmPallet;
}

parameter_types! {
	pub const CollectiveBodyId: BodyId = BodyId::Unit;
}

parameter_types! {
	pub const CouncilBodyId: BodyId = BodyId::Executive;
}

/// Type to convert the council origin to a Plurality `MultiLocation` value.
pub type CouncilToPlurality = BackingToPlurality<
	RuntimeOrigin,
	pallet_collective::Origin<Runtime, CouncilCollective>,
	CouncilBodyId,
>;

/// Type to convert an `Origin` type value into a `MultiLocation` value which represents an interior location
/// of this chain.
pub type LocalOriginToLocation = (
	// We allow an origin from the Collective pallet to be used in XCM as a corresponding Plurality of the
	// `Unit` body.
	CouncilToPlurality,
	// And a usual Signed origin to be used in XCM as a corresponding AccountId32
	SignedToAccountId32<RuntimeOrigin, AccountId, CordNetwork>,
);

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	// Anyone can execute XCM messages locally.
	type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Nothing;
	type XcmExecutor = xcm_executor::XcmExecutor<XcmConfig>;
	// Anyone is able to use teleportation regardless of who they are and what they want to teleport.
	type XcmTeleportFilter = Everything;
	// Anyone is able to use reserve transfers regardless of who they are and what they want to
	// transfer.
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
	type LocationInverter = LocationInverter<Ancestry>;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}
