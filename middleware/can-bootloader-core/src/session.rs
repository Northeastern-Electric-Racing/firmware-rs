//! Choose application startup or a CAN update session from the persisted boot state.

use embassy_boot::{BlockingFirmwareUpdater, State};
use embedded_storage::nor_flash::NorFlash;

use crate::handlers::Handlers;
use crate::image;
use crate::link::Link;
use crate::protocol::*;
use crate::transport::{BootloaderCanIds, Frame, Transport};
use crate::types::{BootAction, Context};

/// Serve commands until the host requests application startup or reset.
/// The caller owns the final reset or interrupt cleanup and jump.
pub async fn run<T, DFU, STATE>(
    transport: &mut T,
    ids: BootloaderCanIds,
    updater: &mut BlockingFirmwareUpdater<'_, DFU, STATE>,
    boot_state: &State,
) -> BootAction
where
    T: Transport,
    DFU: NorFlash,
    STATE: NorFlash,
{
    if *boot_state == State::DfuDetach {
        // Consume entry once; stay here now, but let a later reset boot the old app.
        if updater.mark_booted().is_err() {
            defmt::error!("Failed to clear bootloader entry request; staying in update mode");
        }
    } else if image::active_valid() {
        return BootAction::StartApp;
    }

    let mut link = Link::new(transport, ids);
    let mut handlers = Handlers::new();
    let mut context = Context::new();

    // Serve updates after a DFU request or recover an invalid active image.
    loop {
        let frame = link.receive().await;
        if let Some(action) =
            dispatch(&frame, &mut link, updater, &mut handlers, &mut context).await
        {
            return action;
        }
    }
}

/// Dispatch one request frame; data packets are consumed inside WRITE_MEMORY.
/// Returns an action only when the command ends the bootloader session.
async fn dispatch<T, DFU, STATE>(
    frame: &Frame,
    link: &mut Link<'_, T>,
    updater: &mut BlockingFirmwareUpdater<'_, DFU, STATE>,
    handlers: &mut Handlers,
    context: &mut Context,
) -> Option<BootAction>
where
    T: Transport,
    DFU: NorFlash,
    STATE: NorFlash,
{
    let Some(&command) = frame.data().first() else {
        return None;
    };
    if frame.id() != link.ids.request {
        return None;
    }

    match command {
        GET_INFO => handlers.get_info(link).await,
        GET_STATUS => handlers.get_status(link, context).await,
        START_UPDATE => handlers.start_update(frame, link, context).await,
        WRITE_MEMORY => handlers.write_memory(frame, link, updater, context).await,
        COMPUTE_CRC => handlers.compute_crc(link, updater, context).await,
        ACTIVATE => return handlers.activate(link, updater, context).await,
        START_APP => return handlers.start_app(link).await,
        command => {
            let _ = link.nack(command).await;
        }
    }

    None
}
