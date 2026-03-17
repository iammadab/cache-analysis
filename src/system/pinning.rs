// Pins the current process/thread to a specific Linux CPU core using sched_setaffinity.
pub fn pin_to_core(core_id: usize) -> Result<(), std::io::Error> {
    let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
    unsafe {
        libc::CPU_ZERO(&mut set);
        libc::CPU_SET(core_id, &mut set);

        let rc = libc::sched_setaffinity(
            0,
            std::mem::size_of::<libc::cpu_set_t>(),
            &set as *const libc::cpu_set_t,
        );

        if rc == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}
