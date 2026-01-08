//! Test data for Editor Dev

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ReflectPlanetesComponent;

#[derive(Component, Reflect, Default, Serialize, Deserialize, Debug)]
#[reflect(Component, PlanetesComponent, Default, Serialize, Deserialize)]
pub struct ThingyUnit;

#[derive(Component, Reflect, Serialize, Deserialize, Debug, Default)]
#[reflect(Component, PlanetesComponent, Default, Serialize, Deserialize)]
pub struct ThingyStruct {
    pub field1: String,
    pub field2: i32,
    pub field3: bool,
}

#[derive(Component, Reflect, Default, Serialize, Deserialize, Debug)]
#[reflect(Component, PlanetesComponent, Default, Serialize, Deserialize)]
pub struct ThingyTuple(pub u32, pub u32);

#[derive(Component, Reflect, Default, Serialize, Deserialize, Debug)]
#[reflect(Component, PlanetesComponent, Default, Serialize, Deserialize)]
pub enum ThingyEnum {
    #[default]
    Unit,
    Tuple(ThingyTuple),
}
