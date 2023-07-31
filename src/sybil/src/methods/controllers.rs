use anyhow::Result;
use ic_cdk::{query, update};

use crate::{
    types::{
        config::{Cfg, UpdateCfg},
        state,
    },
    utils::validate_caller,
};

#[update]
pub fn update_cfg(cfg: UpdateCfg) -> Result<(), String> {
    _update_cfg(cfg).map_err(|e| format!("{e:?}"))
}

#[inline(always)]
fn _update_cfg(cfg: UpdateCfg) -> Result<()> {
    validate_caller()?;
    state::update(&cfg);
    Ok(())
}

#[query]
pub fn get_cfg() -> Result<Cfg, String> {
    _get_cfg().map_err(|e| format!("{e:?}"))
}

#[inline(always)]
fn _get_cfg() -> Result<Cfg> {
    validate_caller()?;
    Ok(state::get_cfg())
}

#[update]
pub fn clear_state() -> Result<(), String> {
    _clear_state().map_err(|e| format!("{e:?}"))
}

#[inline(always)]
fn _clear_state() -> Result<()> {
    validate_caller()?;
    state::clear();
    Ok(())
}
