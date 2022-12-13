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

use substrate_wasm_builder::WasmBuilder;

fn main() {
	WasmBuilder::new()
		.with_current_project()
		.import_memory()
		.export_heap_base()
		.build()
}
