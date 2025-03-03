#![no_main]
#![no_std]

#[macro_use]
mod macros;
mod keymap;
mod vial;

use core::cell::RefCell;

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    gpio::{AnyPin, Input, Output},
    interrupt::InterruptExt,
    nvmc::Nvmc,
    peripherals,
    usb::{self, vbus_detect::HardwareVbusDetect, Driver},
};
use keymap::{get_default_keymap, COL, ROW};
use panic_probe as _;
use rmk::{
    bind_device_and_processor_and_run,
    config::{ControllerConfig, RmkConfig, VialConfig},
    debounce::{default_bouncer::DefaultDebouncer, DebouncerTrait},
    futures::future::join,
    input_device::{InputDevice, InputProcessor},
    keyboard::Keyboard,
    keymap::KeyMap,
    light::LightController,
    matrix::Matrix,
    run_rmk,
    storage::{async_flash_wrapper, Storage},
};
use vial::{VIAL_KEYBOARD_DEF, VIAL_KEYBOARD_ID};

bind_interrupts!(struct Irqs {
    USBD => usb::InterruptHandler<peripherals::USBD>;
    CLOCK_POWER => usb::vbus_detect::InterruptHandler;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("RMK start!");
    // Initialize peripherals
    let mut config = ::embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = ::embassy_nrf::interrupt::Priority::P3;
    config.time_interrupt_priority = ::embassy_nrf::interrupt::Priority::P3;
    ::embassy_nrf::interrupt::USBD.set_priority(::embassy_nrf::interrupt::Priority::P2);
    ::embassy_nrf::interrupt::CLOCK_POWER.set_priority(::embassy_nrf::interrupt::Priority::P2);
    let p = ::embassy_nrf::init(config);
    info!("Enabling ext hfosc...");
    ::embassy_nrf::pac::CLOCK.tasks_hfclkstart().write_value(1);
    while ::embassy_nrf::pac::CLOCK.events_hfclkstarted().read() != 1 {}

    // Usb config
    let driver = Driver::new(p.USBD, Irqs, HardwareVbusDetect::new(Irqs));

    // Pin config
    let (input_pins, output_pins) = config_matrix_pins_nrf!(peripherals: p, input: [P0_07, P0_08, P0_11, P0_12], output: [P0_13, P0_14, P0_15]);

    // Use internal flash to emulate eeprom
    let flash = Nvmc::new(p.NVMC);

    // RMK config
    let rmk_config = RmkConfig {
        vial_config: VialConfig::new(VIAL_KEYBOARD_ID, VIAL_KEYBOARD_DEF),
        ..Default::default()
    };

    // Create the debouncer, use COL2ROW by default
    let debouncer = DefaultDebouncer::<ROW, COL>::new();

    // Keyboard matrix, use COL2ROW by default
    let mut matrix = Matrix::<_, _, _, ROW, COL>::new(input_pins, output_pins, debouncer);

    let mut km = get_default_keymap();
    let mut storage = Storage::new(
        async_flash_wrapper(flash),
        &mut keymap::get_default_keymap(),
        rmk_config.storage_config,
    )
    .await;
    let keymap = RefCell::new(
        KeyMap::new_from_storage(
            &mut km,
            Some(&mut storage),
            rmk_config.behavior_config.clone(),
        )
        .await,
    );
    let mut keyboard = Keyboard::new(&keymap, rmk_config.behavior_config.clone());

    let light_controller: LightController<Output> =
        LightController::new(ControllerConfig::default().light_config);

    join(
        bind_device_and_processor_and_run!((matrix) => keyboard),
        run_rmk(&keymap, driver, storage, light_controller, rmk_config),
    )
    .await;
}
