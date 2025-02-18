#![allow(non_camel_case_types)]
use crate::ctx::WasiCtx;
use crate::fdentry::Descriptor;
use crate::memory::*;
use crate::sys::hostcalls_impl;
use crate::{host, wasm32, Error, Result};
use log::trace;
use std::convert::TryFrom;

pub(crate) fn args_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    argv_ptr: wasm32::uintptr_t,
    argv_buf: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "args_get(argv_ptr={:#x?}, argv_buf={:#x?})",
        argv_ptr,
        argv_buf,
    );

    let mut argv_buf_offset = 0;
    let mut argv = vec![];

    for arg in wasi_ctx.args.iter() {
        let arg_bytes = arg.as_bytes_with_nul();
        let arg_ptr = argv_buf + argv_buf_offset;

        enc_slice_of(memory, arg_bytes, arg_ptr)?;

        argv.push(arg_ptr);

        let len = wasm32::uintptr_t::try_from(arg_bytes.len())?;
        argv_buf_offset = argv_buf_offset.checked_add(len).ok_or(Error::EOVERFLOW)?;
    }

    enc_slice_of(memory, argv.as_slice(), argv_ptr)
}

pub(crate) fn args_sizes_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    argc_ptr: wasm32::uintptr_t,
    argv_buf_size_ptr: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "args_sizes_get(argc_ptr={:#x?}, argv_buf_size_ptr={:#x?})",
        argc_ptr,
        argv_buf_size_ptr,
    );

    let argc = wasi_ctx.args.len();
    let argv_size = wasi_ctx
        .args
        .iter()
        .map(|arg| arg.as_bytes_with_nul().len())
        .sum();

    trace!("     | *argc_ptr={:?}", argc);

    enc_usize_byref(memory, argc_ptr, argc)?;

    trace!("     | *argv_buf_size_ptr={:?}", argv_size);

    enc_usize_byref(memory, argv_buf_size_ptr, argv_size)
}

pub(crate) fn environ_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    environ_ptr: wasm32::uintptr_t,
    environ_buf: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "environ_get(environ_ptr={:#x?}, environ_buf={:#x?})",
        environ_ptr,
        environ_buf,
    );

    let mut environ_buf_offset = 0;
    let mut environ = vec![];

    for pair in wasi_ctx.env.iter() {
        let env_bytes = pair.as_bytes_with_nul();
        let env_ptr = environ_buf + environ_buf_offset;

        enc_slice_of(memory, env_bytes, env_ptr)?;

        environ.push(env_ptr);

        let len = wasm32::uintptr_t::try_from(env_bytes.len())?;
        environ_buf_offset = environ_buf_offset
            .checked_add(len)
            .ok_or(Error::EOVERFLOW)?;
    }

    enc_slice_of(memory, environ.as_slice(), environ_ptr)
}

pub(crate) fn environ_sizes_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    environ_count_ptr: wasm32::uintptr_t,
    environ_size_ptr: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "environ_sizes_get(environ_count_ptr={:#x?}, environ_size_ptr={:#x?})",
        environ_count_ptr,
        environ_size_ptr,
    );

    let environ_count = wasi_ctx.env.len();
    let environ_size = wasi_ctx
        .env
        .iter()
        .try_fold(0, |acc: u32, pair| {
            acc.checked_add(pair.as_bytes_with_nul().len() as u32)
        })
        .ok_or(Error::EOVERFLOW)?;

    trace!("     | *environ_count_ptr={:?}", environ_count);

    enc_usize_byref(memory, environ_count_ptr, environ_count)?;

    trace!("     | *environ_size_ptr={:?}", environ_size);

    enc_usize_byref(memory, environ_size_ptr, environ_size as usize)
}

pub(crate) fn random_get(
    memory: &mut [u8],
    buf_ptr: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
) -> Result<()> {
    use rand::{thread_rng, RngCore};

    trace!("random_get(buf_ptr={:#x?}, buf_len={:?})", buf_ptr, buf_len);

    let buf = dec_slice_of_mut::<u8>(memory, buf_ptr, buf_len)?;

    thread_rng().fill_bytes(buf);

    Ok(())
}

pub(crate) fn clock_res_get(
    memory: &mut [u8],
    clock_id: wasm32::__wasi_clockid_t,
    resolution_ptr: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "clock_res_get(clock_id={:?}, resolution_ptr={:#x?})",
        clock_id,
        resolution_ptr,
    );

    let clock_id = dec_clockid(clock_id);
    let resolution = hostcalls_impl::clock_res_get(clock_id)?;

    trace!("     | *resolution_ptr={:?}", resolution);

    enc_timestamp_byref(memory, resolution_ptr, resolution)
}

pub(crate) fn clock_time_get(
    memory: &mut [u8],
    clock_id: wasm32::__wasi_clockid_t,
    precision: wasm32::__wasi_timestamp_t,
    time_ptr: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "clock_time_get(clock_id={:?}, precision={:?}, time_ptr={:#x?})",
        clock_id,
        precision,
        time_ptr,
    );

    let clock_id = dec_clockid(clock_id);
    let time = hostcalls_impl::clock_time_get(clock_id)?;

    trace!("     | *time_ptr={:?}", time);

    enc_timestamp_byref(memory, time_ptr, time)
}

pub(crate) fn sched_yield() -> Result<()> {
    trace!("sched_yield()");

    std::thread::yield_now();

    Ok(())
}

pub(crate) fn poll_oneoff(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    input: wasm32::uintptr_t,
    output: wasm32::uintptr_t,
    nsubscriptions: wasm32::size_t,
    nevents: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "poll_oneoff(input={:#x?}, output={:#x?}, nsubscriptions={}, nevents={:#x?})",
        input,
        output,
        nsubscriptions,
        nevents,
    );

    if u64::from(nsubscriptions) > wasm32::__wasi_filesize_t::max_value() {
        return Err(Error::EINVAL);
    }

    enc_pointee(memory, nevents, 0)?;

    let input_slice = dec_slice_of::<wasm32::__wasi_subscription_t>(memory, input, nsubscriptions)?;
    let subscriptions = input_slice
        .into_iter()
        .map(dec_subscription)
        .collect::<Result<Vec<_>>>()?;
    let output_slice = dec_slice_of_mut::<wasm32::__wasi_event_t>(memory, output, nsubscriptions)?;
    let mut output_slice_iter = output_slice.iter_mut();
    let mut events_count = 0;

    let mut timeout: Option<ClockEventData> = None;
    let mut fd_events = Vec::new();
    for subscription in subscriptions {
        match subscription.type_ {
            host::__WASI_EVENTTYPE_CLOCK => {
                let clock = unsafe { subscription.u.clock };
                let delay = wasi_clock_to_relative_ns_delay(clock)?;

                log::debug!("poll_oneoff event.u.clock = {:?}", clock);
                log::debug!("poll_oneoff delay = {:?}ns", delay);

                let current = ClockEventData {
                    delay,
                    userdata: subscription.userdata,
                };
                let timeout = timeout.get_or_insert(current);
                if current.delay < timeout.delay {
                    *timeout = current;
                }
            }
            type_
                if type_ == host::__WASI_EVENTTYPE_FD_READ
                    || type_ == host::__WASI_EVENTTYPE_FD_WRITE =>
            {
                let wasi_fd = unsafe { subscription.u.fd_readwrite.fd };
                let rights = if type_ == host::__WASI_EVENTTYPE_FD_READ {
                    host::__WASI_RIGHT_FD_READ
                } else {
                    host::__WASI_RIGHT_FD_WRITE
                };

                match unsafe {
                    wasi_ctx
                        .get_fd_entry(wasi_fd)
                        .and_then(|fe| fe.as_descriptor(rights, 0))
                } {
                    Ok(descriptor) => fd_events.push(FdEventData {
                        descriptor,
                        type_: subscription.type_,
                        userdata: subscription.userdata,
                    }),
                    Err(err) => {
                        let event = host::__wasi_event_t {
                            userdata: subscription.userdata,
                            type_,
                            error: err.as_wasi_errno(),
                            u: host::__wasi_event_t___wasi_event_u {
                                fd_readwrite: host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                                    nbytes: 0,
                                    flags: 0,
                                }
                            }
                        };
                        *output_slice_iter
                            .next()
                            .expect("number of subscriptions has to match number of events") =
                            enc_event(event);
                        events_count += 1;
                    }
                };
            }
            _ => unreachable!(),
        }
    }

    log::debug!("poll_oneoff timeout = {:?}", timeout);
    log::debug!("poll_oneoff fd_events = {:?}", fd_events);

    let events = hostcalls_impl::poll_oneoff(timeout, fd_events)?;
    events_count += events.len();
    for event in events {
        *output_slice_iter
            .next()
            .expect("number of subscriptions has to match number of events") = enc_event(event);
    }

    trace!("     | *nevents={:?}", events_count);

    enc_pointee(memory, nevents, events_count)
}

fn wasi_clock_to_relative_ns_delay(
    wasi_clock: host::__wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
) -> Result<u128> {
    use std::time::SystemTime;

    if wasi_clock.flags != host::__WASI_SUBSCRIPTION_CLOCK_ABSTIME {
        return Ok(u128::from(wasi_clock.timeout));
    }
    let now: u128 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| Error::ENOTCAPABLE)?
        .as_nanos();
    let deadline = u128::from(wasi_clock.timeout);
    Ok(deadline.saturating_sub(now))
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct ClockEventData {
    pub(crate) delay: u128, // delay is expressed in nanoseconds
    pub(crate) userdata: host::__wasi_userdata_t,
}

#[derive(Debug)]
pub(crate) struct FdEventData<'a> {
    pub(crate) descriptor: &'a Descriptor,
    pub(crate) type_: host::__wasi_eventtype_t,
    pub(crate) userdata: host::__wasi_userdata_t,
}
