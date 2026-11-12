// SPDX-License-Identifier: GPL-3.0-only
use fm1_hal::usb_queues::{self, Io};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct State {
    words: [u32; 64],
    write: u32,
    read: u32,
}
struct Hook {
    count: Cell<usize>,
    at: usize,
    fired: Cell<bool>,
    produce: bool,
    accepted: Cell<bool>,
    output: Rc<RefCell<Vec<u32>>>,
}
struct Proxy {
    state: Rc<RefCell<State>>,
    hook: Option<Rc<Hook>>,
}
impl Proxy {
    fn step(&self) {
        if let Some(hook) = &self.hook {
            let count = hook.count.get();
            hook.count.set(count + 1);
            if count == hook.at && !hook.fired.replace(true) {
                let mut peer = Self {
                    state: self.state.clone(),
                    hook: None,
                };
                if hook.produce {
                    hook.accepted.set(usb_queues::push(&mut peer, true, 0xfeed));
                } else {
                    usb_queues::pop(&mut peer, |v| hook.output.borrow_mut().push(v));
                }
            }
        }
        let state = self.state.borrow();
        assert!(state.write.wrapping_sub(state.read) <= 64);
    }
}
impl Io for Proxy {
    fn write_index(&self) -> u32 {
        let v = self.state.borrow().write;
        self.step();
        v
    }
    fn read_index(&self) -> u32 {
        let v = self.state.borrow().read;
        self.step();
        v
    }
    fn set_write(&mut self, v: u32) {
        self.state.borrow_mut().write = v;
        self.step();
    }
    fn set_read(&mut self, v: u32) {
        self.state.borrow_mut().read = v;
        self.step();
    }
    fn word(&self, i: usize) -> u32 {
        let v = self.state.borrow().words[i];
        self.step();
        v
    }
    fn put(&mut self, i: usize, v: u32) {
        self.state.borrow_mut().words[i] = v;
        self.step();
    }
    fn publish(&mut self) {
        self.step();
    }
}
#[test]
fn every_scalar_slot_and_publication_boundary_preserves_fifo_during_irq_role_interleaving() {
    for base in [0u32, 63, u32::MAX - 63, u32::MAX - 3] {
        for fill in 0..=64u32 {
            for produce in [false, true] {
                for at in 0..7 {
                    let mut words = [0xdead0000; 64];
                    for i in 0..fill {
                        words[(base.wrapping_add(i) % 64) as usize] = i;
                    }
                    let state = Rc::new(RefCell::new(State {
                        words,
                        write: base.wrapping_add(fill),
                        read: base,
                    }));
                    let output = Rc::new(RefCell::new(Vec::new()));
                    let hook = Rc::new(Hook {
                        count: Cell::new(0),
                        at,
                        fired: Cell::new(false),
                        produce,
                        accepted: Cell::new(false),
                        output: output.clone(),
                    });
                    let mut proxy = Proxy {
                        state: state.clone(),
                        hook: Some(hook.clone()),
                    };
                    let accepted = if produce {
                        usb_queues::pop(&mut proxy, |v| output.borrow_mut().push(v));
                        hook.accepted.get()
                    } else {
                        usb_queues::push(&mut proxy, true, 0xbeef)
                    };
                    let mut peer = Proxy { state, hook: None };
                    while usb_queues::pop(&mut peer, |v| output.borrow_mut().push(v)) {}
                    let mut expected: Vec<u32> = (0..fill).collect();
                    if accepted {
                        expected.push(if produce { 0xfeed } else { 0xbeef });
                    }
                    assert_eq!(
                        *output.borrow(),
                        expected,
                        "base{base} fill{fill} producer{produce} afterIO{at}"
                    );
                }
            }
        }
    }
}
