// Copyright 2019-2023 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::utils::io::random_access::RandomAccessFile;
use crate::{
    daemon::bundle::load_actor_bundles,
    networks::{ChainConfig, Height, NetworkChain},
    shim::state_tree::StateRoot,
    state_migration::run_state_migrations,
};
use anyhow::*;
use cid::Cid;
use fvm_ipld_encoding::CborStore;
use pretty_assertions::assert_eq;
use std::path::PathBuf;
use std::{str::FromStr, sync::Arc};
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn test_nv17_state_migration_calibnet() -> Result<()> {
    // forest_filecoin::state_migration: State migration at height Shark(epoch 16800) was successful,
    // Previous state: bafy2bzacedxtdhqjsrw2twioyaeomdk4z7umhgfv36vzrrotjb4woutphqgyg,
    // new state: bafy2bzacecrejypa2rqdh3geg2u3qdqdrejrfqvh2ykqcrnyhleehpiynh4k4.
    //
    // See <https://github.com/ChainSafe/forest/actions/runs/5579505385/jobs/10195488001#step:6:232>
    test_state_migration(
        Height::Shark,
        NetworkChain::Calibnet,
        Cid::from_str("bafy2bzacedxtdhqjsrw2twioyaeomdk4z7umhgfv36vzrrotjb4woutphqgyg")?,
        Cid::from_str("bafy2bzacecrejypa2rqdh3geg2u3qdqdrejrfqvh2ykqcrnyhleehpiynh4k4")?,
    )
    .await
}

#[ignore = "flaky"]
#[tokio::test]
async fn test_nv18_state_migration_calibnet() -> Result<()> {
    // State migration at height Hygge(epoch 322354) was successful,
    // Previous state: bafy2bzacedjqwdqxlkyyuohmtcfciekl5qh2s4yf67neiuuhkibbteqoucvsm,
    // new state: bafy2bzacedhhgkmr26rbr3yujounnz2ufiwrlvamogyabgfv6uvwq3rlv4t2i.
    //
    // See <https://github.com/ChainSafe/forest/actions/runs/5579505385/jobs/10195488001#step:6:515>
    test_state_migration(
        Height::Hygge,
        NetworkChain::Calibnet,
        Cid::from_str("bafy2bzacedjqwdqxlkyyuohmtcfciekl5qh2s4yf67neiuuhkibbteqoucvsm")?,
        Cid::from_str("bafy2bzacedhhgkmr26rbr3yujounnz2ufiwrlvamogyabgfv6uvwq3rlv4t2i")?,
    )
    .await
}

#[ignore = "flaky"]
#[tokio::test]
async fn test_nv19_state_migration_calibnet() -> Result<()> {
    // State migration at height Lightning(epoch 489094) was successful,
    // Previous state: bafy2bzacedgamjgha75e7w2cgklfdgtmumsj7nadqppnpz3wexl2wl6dexsle,
    // new state: bafy2bzacebhjx4uqtg6c65km46wiiq45dbbeckqhs2oontwdzba335nxk6bia.
    //
    // See <https://github.com/ChainSafe/forest/actions/runs/5579505385/jobs/10195488001#step:6:232>
    test_state_migration(
        Height::Lightning,
        NetworkChain::Calibnet,
        Cid::from_str("bafy2bzacedgamjgha75e7w2cgklfdgtmumsj7nadqppnpz3wexl2wl6dexsle")?,
        Cid::from_str("bafy2bzacebhjx4uqtg6c65km46wiiq45dbbeckqhs2oontwdzba335nxk6bia")?,
    )
    .await
}

async fn test_state_migration(
    height: Height,
    network: NetworkChain,
    old_state: Cid,
    expected_new_state: Cid,
) -> Result<()> {
    // Car files are cached under data folder for Go test to pick up without network access
    let car_path = PathBuf::from(format!("./src/state_migration/tests/data/{old_state}.car"));
    if !car_path.is_file() {
        let tmp: tempfile::TempPath =
            tempfile::NamedTempFile::new_in(car_path.parent().unwrap())?.into_temp_path();
        {
            let mut reader = crate::utils::net::reader(&format!(
                "https://forest-continuous-integration.fra1.cdn.digitaloceanspaces.com/state_migration/state/{old_state}.car"
            ))
            .await?;
            let mut writer = tokio::io::BufWriter::new(tokio::fs::File::create(&tmp).await?);
            tokio::io::copy(&mut reader, &mut writer).await?;
            writer.shutdown().await?;
        }
        tmp.persist(&car_path)?;
    }

    let store = Arc::new(crate::db::car::plain::PlainCar::new(
        RandomAccessFile::open(&car_path)?,
    )?);
    load_actor_bundles(&store).await?;

    let chain_config = Arc::new(ChainConfig::from_chain(&network));
    let height_info = &chain_config.height_infos[height as usize];

    let state_root: StateRoot = store.get_cbor(&old_state)?.unwrap();
    println!("Actor root (for Go test): {}", state_root.actors);

    let new_state = run_state_migrations(height_info.epoch, &chain_config, &store, &old_state)?;

    assert_eq!(new_state, Some(expected_new_state));

    Ok(())
}
