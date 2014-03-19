use std::{str, vec};
use std::cmp::min;
use std::io::{Buffer, IoError, IoResult};
use std::vec_ng::Vec;

pub struct LookaheadBuffer<'a> {
    priv buf: &'a mut Buffer,
    priv saved: Vec<u8>,
    priv savedpos: uint,
    priv savederr: Option<IoError>,
}

impl<'a> LookaheadBuffer<'a> {
    pub fn new(buf: &'a mut Buffer) -> LookaheadBuffer<'a> {
        LookaheadBuffer { buf: buf, saved: Vec::new(), savedpos: 0, savederr: None }
    }

    pub fn fill_request<'a>(&'a mut self, amt: uint) -> IoResult<&'a [u8]> {
        if self.savedpos == self.saved.len() {
            // give a saved error if any
            match self.savederr.take() {
                Some(err) => { return Err(err); }
                None => {}
            }

            // try *not* to use the `saved` buffer if possible
            // we have no buffers to return in front of it, so we can directly give the error
            let earlyret = {
                let buf = try!(self.buf.fill());
                if buf.len() >= amt {
                    true
                } else {
                    self.saved.clear();
                    self.saved.push_all(buf);
                    self.savedpos = 0;
                    false
                }
            };
            if earlyret {
                // we can't borrow `buf` this longer...
                return Ok(try!(self.buf.fill()));
            }
        } else if self.savedpos > 0 {
            // TODO amortize this: we need to occasionally shrink the `saved` buffer,
            // otherwise we may hit the pathological case when the caller repeatedly
            // request the large amount of buffers, but we can't always do this
            // since it will significantly degrade the typical performance.
            //
            //for i in range(self.savedpos, self.saved.len()) {
            //    self.saved[i] = self.saved[i - self.savedpos];
            //}
            //self.savedpos = 0;
            }

        // only call `fill` when the `saved` buffer is not enough
        let minlen = self.savedpos + amt;
        if self.saved.len() < minlen {
            // give a saved error if any
            match self.savederr.take() {
                Some(err) => { return Err(err); }
                None => {}
            }

            while self.saved.len() < minlen {
                match self.buf.fill() {
                    Ok(buf) => {
                        self.saved.push_all(buf);
                    }
                    Err(err) => {
                        self.savederr = Some(err);
                        break;
                    }
                }
            }
        }

        Ok(self.saved.slice_from(self.savedpos))
    }

    pub fn read_pad_char(&mut self, pad: char) -> IoResult<uint> {
        if (pad as uint) < 128 { // optimization
            let pad = pad as u8;
            return self.read_pad_byte_if(|ch| ch == pad);
        }

        let mut padbuf = [0u8, ..4];
        let padlen = pad.encode_utf8(padbuf.as_mut_slice());

        let mut consume;
        let mut consumed = 0;
        'reading: loop {
            {
                let buf = try!(self.fill_request(padlen));
                if buf.len() < padlen {
                    // the remaining bytes cannot be equal to `padbuf`, so we are done.
                    return Ok(consumed);
                }

                // we intentionally leave the last `buf.len() % padlen` bytes;
                // these bytes should be checked after the next call to `fill_request`.
                let nchars = buf.len() / padlen;
                let upto = nchars * padlen;
                let mut offset = 0;
                for (i, &ch) in buf.slice_to(upto).iter().enumerate() {
                    if padbuf[offset] != ch {
                        consume = i - offset;
                        consumed += consume / padlen;
                        break 'reading;
                    }
                    offset += 1;
                    if offset == padlen { offset = 0; }
                }

                consume = upto;
                consumed += nchars;
            }
            self.consume(consume);
        }
        self.consume(consume);
        Ok(consumed)
    }

    pub fn read_pad_byte_if(&mut self, is_pad: |u8| -> bool) -> IoResult<uint> {
        let mut consume;
        let mut consumed = 0;
        'reading: loop {
            {
                let buf = try!(self.fill_request(1));
                if buf.is_empty() { // no read possible, error has been saved
                    return Ok(consumed);
                }

                for (i, &ch) in buf.iter().enumerate() {
                    if !is_pad(ch) {
                        consume = i;
                        consumed += consume;
                        break 'reading;
                    }
                }

                consume = buf.len();
                consumed += consume;
            }
            self.consume(consume);
        }
        self.consume(consume);
        Ok(consumed)
    }

    pub fn peek_byte(&mut self) -> IoResult<Option<u8>> {
        let buf = try!(self.fill_request(1));
        if buf.is_empty() { return Ok(None); }
        Ok(Some(buf[0]))
    }

    pub fn peek_char(&mut self) -> IoResult<Option<char>> {
        let width;
        {
            let buf = try!(self.fill_request(1));
            if buf.is_empty() { return Ok(None); }
            let first_byte = buf[0];
            width = str::utf8_char_width(first_byte);
            if width == 1 { return Ok(Some(first_byte as char)); }
            if width == 0 { return Ok(None); }
        }

        let buf = try!(self.fill_request(width));
        if buf.len() < width { return Ok(None); }
        match str::from_utf8(buf.slice_to(width)) {
            Some(s) => Ok(Some(s.char_at(0))),
            None => Ok(None),
        }
    }
}

impl<'a> Reader for LookaheadBuffer<'a> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
        let len;
        {
            let filled = try!(self.fill_request(0));
            len = min(buf.len(), filled.len());
            let input = filled.slice(0, len);
            let output = buf.mut_slice(0, len);
            vec::bytes::copy_memory(output, input);
        }
        self.consume(len);
        Ok(len)
    }
}

impl<'a> Buffer for LookaheadBuffer<'a> {
    fn fill<'a>(&'a mut self) -> IoResult<&'a [u8]> {
        self.fill_request(0)
    }

    fn consume(&mut self, amt: uint) {
        if self.savedpos == self.saved.len() {
            self.buf.consume(amt);
        } else {
            self.savedpos += amt;
            assert!(self.savedpos <= self.saved.len());
        }
    }
}

