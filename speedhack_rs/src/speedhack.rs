use once_cell::sync::Lazy;
use retour::static_detour;
use std::sync::RwLock;
use windows::Win32::System::Performance::QueryPerformanceCounter;
use windows::Win32::System::SystemInformation;
use windows_sys::Win32::Foundation::{BOOL, TRUE};

pub static MANAGER: Lazy<RwLock<SpeedHackManager>> = Lazy::new(|| unsafe { SpeedHackManager::new().unwrap().into() });

pub struct SpeedHackManager {
    speed: f64,

    qpc_basetime: i64,
    qpc_offset_time: i64,
}

static_detour! {
    pub static _QUERY_PERFORMANCE_COUNTER: unsafe extern "system" fn(*mut i64) -> BOOL;
}

impl SpeedHackManager {
    pub unsafe fn new() -> anyhow::Result<Self> {
        let mut qpc_basetime = 0i64;
        QueryPerformanceCounter(&mut qpc_basetime)?;

        _QUERY_PERFORMANCE_COUNTER.initialize(
            windows_sys::Win32::System::Performance::QueryPerformanceCounter,
            real_query_performance_counter,
        )?;

        _QUERY_PERFORMANCE_COUNTER.enable()?;

        Ok(SpeedHackManager {
            speed: 1.0,
            qpc_basetime,
            qpc_offset_time: qpc_basetime,
        })
    }

    /// Disable the static detours
    pub fn detach(&mut self) -> anyhow::Result<()> {
        unsafe {
            _QUERY_PERFORMANCE_COUNTER.disable()?;
        }

        Ok(())
    }

    pub fn set_speed(&mut self, speed: f64) {
        // Update the offsets to ensure we don't cause negative time warps.
        unsafe {
            self.qpc_offset_time = self.get_performance_counter();
            let _ = _QUERY_PERFORMANCE_COUNTER.call(&mut self.qpc_basetime);
        }

        self.speed = speed;
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    pub fn get_performance_counter(&self) -> i64 {
        let mut temp = 0i64;

        unsafe {
            _QUERY_PERFORMANCE_COUNTER.call(&mut temp);
            self.qpc_offset_time + ((temp - self.qpc_basetime) as f64 * self.speed) as i64
        }
    }
}

fn real_query_performance_counter(lp_performance_counter: *mut i64) -> BOOL {
    unsafe {
        *lp_performance_counter = MANAGER.read().unwrap().get_performance_counter();
    }

    TRUE
}

impl Drop for SpeedHackManager {
    fn drop(&mut self) {
        if let Err(e) = self.detach() {
            log::error!("Failed to detach SpeedHackManager due to {:?}", e);
        }
    }
}
