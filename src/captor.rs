//! This is file is based on example code provided by the `pipewire` crate in `https://docs.rs/crate/pipewire/latest/source/examples/audio-capture.rs`
//!
//! // Copyright The pipewire-rs Contributors.
//! // SPDX-License-Identifier: MIT
//!
//! //! This file is a rustic interpretation of the [PipeWire audio-capture.c example][example]
//! //!
//! //! example: https://docs.pipewire.org/audio-capture_8c-example.html

use pipewire as pw;

use pipewire::stream::Stream;
use pipewire::sys::pw_buffer;
use pw::{properties::properties, spa};
use spa::param::format::{MediaSubtype, MediaType};
use spa::param::format_utils;
use spa::pod::Pod;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::mem;
use std::slice;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

struct UserData {
    format: spa::param::audio::AudioInfoRaw,
    cursor_move: bool,
    capture_buffer: Vec<f32>,
    //pw_quantum: usize,
    fresh_tx: Sender<VecDeque<f32>>,
    completed_rx: Receiver<VecDeque<f32>>,

    samples_per_frame: usize,
}

/// This is basically useless for current implementation. Will become useful in future. Good to know its working.
fn setup_buffer(user_data: &mut UserData, pw_buffer: *mut pw_buffer, target_framerate: &u16) {
    // PipeWire supplies a valid pw_buffer
    let max_size = unsafe {
        let Some(pw_buffer) = pw_buffer.as_ref() else {
            return;
        };

        let Some(spa_buffer) = pw_buffer.buffer.as_ref() else {
            return;
        };

        if spa_buffer.n_datas == 0 {
            return;
        }

        let Some(datas_ptr) = spa_buffer.datas.as_ref() else {
            return;
        };

        let datas = slice::from_raw_parts(datas_ptr, spa_buffer.n_datas as usize);

        datas
            .iter()
            .map(|data| data.maxsize as usize)
            .max()
            .unwrap_or(0)
    };

    user_data.samples_per_frame = (user_data.format.rate() / *target_framerate as u32) as usize;

    //user_data.pw_quantum = max_size;
    // capture buffer will never be more than max_size longer than samples per frame
    user_data
        .capture_buffer
        .reserve(user_data.samples_per_frame as usize + max_size);
}

fn capture_samples(stream: &Stream, user_data: &mut UserData) {
    match stream.dequeue_buffer() {
        None => println!("out of buffers"),
        Some(mut buffer) => {
            let datas = buffer.datas_mut();
            if datas.is_empty() {
                return;
            }

            let data = &mut datas[0];
            let n_channels = user_data.format.channels() as usize;
            let n_samples = data.chunk().size() as usize;

            // extend the transform buffer with the bytes from each channel cnoverted to f32 and averaged
            if let Some(samples) = data.data() {
                user_data.capture_buffer.extend(
                    samples[..(n_samples * mem::size_of::<f32>())]
                        .chunks_exact(mem::size_of::<f32>() * n_channels)
                        .map(|samples| -> f32 {
                            samples
                                .chunks_exact(mem::size_of::<f32>())
                                .map(|bytes| -> f32 {
                                    f32::from_le_bytes(bytes.try_into().unwrap())
                                })
                                .sum::<f32>()
                                / n_channels as f32
                        }),
                );

                // the main thread should have finished transforming and displaying the data & return the freed slice
                if user_data.capture_buffer.len() > user_data.samples_per_frame
                    && let Ok(mut buffer) = user_data.completed_rx.try_recv()
                {
                    buffer.drain(..(n_samples / n_channels));
                    buffer.extend(user_data.capture_buffer.iter());

                    user_data.capture_buffer.clear();

                    user_data.fresh_tx.send(buffer).unwrap();
                }
            }
        }
    }
}

fn param_changed(user_data: &mut UserData, id: u32, param: Option<&Pod>) {
    // NULL means to clear the format
    let Some(param) = param else {
        return;
    };
    if id != pw::spa::param::ParamType::Format.as_raw() {
        return;
    }

    let (media_type, media_subtype) = match format_utils::parse_format(param) {
        Ok(v) => v,
        Err(_) => return,
    };

    // only accept raw audio
    if media_type != MediaType::Audio || media_subtype != MediaSubtype::Raw {
        return;
    }

    // call a helper function to parse the format for us.
    user_data
        .format
        .parse(param)
        .expect("Failed to parse param changed to AudioInfoRaw");

    println!(
        "capturing rate:{} channels:{}",
        user_data.format.rate(),
        user_data.format.channels()
    );
}

//fn update_max_buffer_size(&mut self, pw_buffer: &mut pw) {}

pub fn run(
    //channel_map: Vec<(Vec<u8>, u8)>,
    target_framerate: u16,
    fresh_tx: Sender<VecDeque<f32>>,
    completed_rx: Receiver<VecDeque<f32>>,
) -> Result<(), pw::Error> {
    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;

    /* Create a simple stream, the simple stream manages the core and remote
     * objects for you if you don't need to deal with them.
     *
     * If you plan to autoconnect your stream, you need to provide at least
     * media, category and role properties.
     *
     * Pass your events and a user_data pointer as the last arguments. This
     * will inform you about the stream state. The most important event
     * you need to listen to is the process event where you need to produce
     * the data.
     */
    #[cfg(not(feature = "v0_3_44"))]
    let props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Music",
    };
    #[cfg(feature = "v0_3_44")]
    let props = {
        let mut props = properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Music",
        };
        if let Some(target) = opt.target {
            props.insert(*pw::keys::TARGET_OBJECT, target);
        }
        props
    };

    // uncomment if you want to capture from the sink monitor ports
    let mut props = props;
    props.insert(*pw::keys::STREAM_CAPTURE_SINK, "true");

    let stream = pw::stream::StreamRc::new(core, "audio-capture", props)?;

    let data = UserData {
        format: Default::default(),
        cursor_move: false,
        capture_buffer: Vec::new(),
        //pw_quantum: 0,
        fresh_tx,
        completed_rx,
        samples_per_frame: 0,
    };

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .add_buffer(move |_, user_data, pw_buffer| {
            setup_buffer(user_data, pw_buffer, &target_framerate)
        })
        .param_changed(|_, user_data, id, param| param_changed(user_data, id, param))
        .process(move |stream, user_data| capture_samples(stream, user_data))
        .register()?;

    /* Make one parameter with the supported formats. The SPA_PARAM_EnumFormat
     * id means that this is a format enumeration (of 1 value).
     * We leave the channels and rate empty to accept the native graph
     * rate and channels. */
    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    let obj = pw::spa::pod::Object {
        type_: pw::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: pw::spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };
    let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .unwrap()
    .0
    .into_inner();

    let mut params = [Pod::from_bytes(&values).unwrap()];

    /* connect this stream. We ask that our process function is
     * called in a realtime thread. */
    stream.connect(
        spa::utils::Direction::Input,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;

    // and wait while we let things run
    mainloop.run();

    Ok(())
}
