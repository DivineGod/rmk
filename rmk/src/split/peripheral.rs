use super::driver::{SplitReader, SplitWriter};
use super::SplitMessage;
use crate::channel::KEY_EVENT_CHANNEL;
use crate::event::Event;
use crate::matrix::MatrixTrait;
#[cfg(not(feature = "_nrf_ble"))]
use crate::split::serial::SerialSplitDriver;
use crate::CONNECTION_STATE;
#[cfg(feature = "_nrf_ble")]
use embassy_executor::Spawner;
use embassy_futures::select::select;
#[cfg(not(feature = "_nrf_ble"))]
use embedded_io_async::{Read, Write};

pub async fn run_peripheral_matrix<M: MatrixTrait>(mut matrix: M) {
    loop {
        let event = matrix.read_event().await;

        if let Event::Key(key_event) = event {
            KEY_EVENT_CHANNEL.send(key_event).await;
        }
    }
}

/// Run the split peripheral service.
///
/// # Arguments
///
/// * `matrix` - the matrix scanning implementation to use.
/// * `central_addr` - (optional) central's BLE static address. This argument is enabled only for nRF BLE split now
/// * `peripheral_addr` - (optional) peripheral's BLE static address. This argument is enabled only for nRF BLE split now
/// * `serial` - (optional) serial port used to send peripheral split message. This argument is enabled only for serial split now
/// * `spawner`: (optional) embassy spawner used to spawn async tasks. This argument is enabled for non-esp microcontrollers
pub async fn run_rmk_split_peripheral<#[cfg(not(feature = "_nrf_ble"))] S: Write + Read>(
    #[cfg(feature = "_nrf_ble")] central_addr: [u8; 6],
    #[cfg(feature = "_nrf_ble")] peripheral_addr: [u8; 6],
    #[cfg(not(feature = "_nrf_ble"))] serial: S,
    #[cfg(feature = "_nrf_ble")] spawner: Spawner,
) {
    #[cfg(not(feature = "_nrf_ble"))]
    {
        let mut peripheral = SplitPeripheral::new(SerialSplitDriver::new(serial));
        loop {
            peripheral.run().await;
        }
    }

    #[cfg(feature = "_nrf_ble")]
    crate::split::nrf::peripheral::initialize_nrf_ble_split_peripheral_and_run(
        central_addr,
        peripheral_addr,
        spawner,
    )
    .await;
}

/// The split peripheral instance.
pub(crate) struct SplitPeripheral<S: SplitWriter + SplitReader> {
    split_driver: S,
}

impl<S: SplitWriter + SplitReader> SplitPeripheral<S> {
    pub(crate) fn new(split_driver: S) -> Self {
        Self { split_driver }
    }

    /// Run the peripheral keyboard service.
    ///
    /// The peripheral uses the general matrix, does scanning and send the key events through `SplitWriter`.
    /// If also receives split messages from the central through `SplitReader`.
    pub(crate) async fn run(&mut self) -> ! {
        loop {
            match select(self.split_driver.read(), KEY_EVENT_CHANNEL.receive()).await {
                embassy_futures::select::Either::First(m) => match m {
                    // Currently only handle the central state message
                    Ok(split_message) => match split_message {
                        SplitMessage::ConnectionState(state) => {
                            info!("Received connection state update: {}", state);
                            CONNECTION_STATE.store(state, core::sync::atomic::Ordering::Release);
                        }
                        _ => (),
                    },
                    Err(e) => {
                        error!("Split message read error: {:?}", e);
                    }
                },
                embassy_futures::select::Either::Second(e) => {
                    // Only send the key event if the connection is established
                    if CONNECTION_STATE.load(core::sync::atomic::Ordering::Acquire) {
                        info!("Writing split message to central");
                        self.split_driver.write(&SplitMessage::Key(e)).await.ok();
                    }
                }
            }
        }
    }
}
