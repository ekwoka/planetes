use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ReflectPlanetesComponent;

#[derive(Component, Reflect, Serialize, Deserialize, Debug)]
#[reflect(Component, PlanetesComponent, Serialize, Deserialize)]
pub struct ThingyUnit;

#[derive(Component, Reflect, Serialize, Deserialize, Debug)]
#[reflect(Component, PlanetesComponent, Serialize, Deserialize)]
pub struct ThingyStruct {
    pub field1: String,
    pub field2: i32,
}

#[derive(Component, Reflect, Serialize, Deserialize, Debug)]
#[reflect(Component, PlanetesComponent, Serialize, Deserialize)]
pub struct ThingyTuple(pub u32, pub u32);

#[derive(Component, Reflect, Serialize, Deserialize, Debug)]
#[reflect(Component, PlanetesComponent, Serialize, Deserialize)]
pub enum ThingyEnum {
    Unit,
    Tuple(ThingyTuple),
}
