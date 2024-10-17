//! Modified functions from uiua's encoding logic:
//! https://github.com/uiua-lang/uiua/blob/95aa14c99a47ccf83bf81433aa4015dd752c2834/src/algorithm/encode.rs
//!
//! uiua is licensed by Kaikalii under the MIT License:
//!
//! MIT License
//!
//! Copyright (c) 2023 Kaikalii
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.

use uiua::Value;

pub fn value_to_audio_channels(audio: &Value) -> Result<Vec<Vec<f32>>, String> {
    // logic taken out of uiua's editor
    // https://github.com/uiua-lang/uiua/blob/95aa14c99a47ccf83bf81433aa4015dd752c2834/pad/editor/src/utils.rs#L931-L932
    // ...with an added check for # of channels and song length because users are evil and will exploit your bot at every given opportunity
    if audio.rank() > 2 {
        return Err(format!(
            "Audio must be a rank 1 or 2 numeric array, but it is rank {}",
            audio.rank()
        ));
    }
    if !audio
        .shape()
        .last()
        .is_some_and(|n| (44100 / 4..44100 * 60).contains(n))
    {
        return Err("Audio too short or too long".into());
    }

    if !matches!(&audio, Value::Num(arr) if arr.elements().all(|x| x.abs() <= 5.0)) {
        return Err("Audio samples are too loud".into());
    }

    let interleaved: Vec<f32> = match audio {
        Value::Num(nums) => nums
            .row_slices()
            .flatten()
            .map(|&f| f as f32 * 0.5 + 0.5)
            .collect(),
        Value::Byte(byte) => byte
            .row_slices()
            .flatten()
            .map(|&b| b as f32 * 0.5 + 0.5)
            .collect(),
        _ => return Err("Audio must be a numeric array".into()),
    };
    let (length, mut channels) = match audio.rank() {
        1 => (interleaved.len(), vec![interleaved]),
        2 => (
            audio.row_len(),
            interleaved
                .chunks_exact(audio.row_len())
                .map(|c| c.to_vec())
                .collect(),
        ),
        _ => {
            // validated at the start
            unreachable!()
        }
    };
    if channels.len() > 5 {
        return Err(format!(
            "Audio can have at most 5 channels, but its shape is {}",
            audio.shape()
        ));
    }

    if channels.is_empty() {
        channels.push(vec![0.0; length]);
    }
    Ok(channels)
}
