//! The host's rules for the actions a behaviour asks for: what `kh1-recom`'s `win::kh1_enemy`
//! (Re:CoM's enemy states running a KH1 behaviour) decides about a running action, as pure rules
//! the DLL calls, so the behaviours' tests can drive a behaviour through the same rules
//! ([`model::Model`], tests only).
//!
//! A steered attack ([`super::Cmd::Attack`], KH1's plain request made with attack rows) plays in
//! its Re:CoM attack state while the behaviour keeps running, polled from that state's step. An
//! attack the behaviour waits on ([`super::Cmd::Action`] with an attack) plays there with the
//! behaviour paused.

/// The running action as the host sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Running {
    pub request: u16,
    /// It plays in a Re:CoM attack state (made with attack rows).
    pub attack: bool,
    /// A steered attack (the behaviour keeps running).
    pub steered: bool,
    /// Its motion plays (an attack: its attack state has begun).
    pub playing: bool,
}

/// Stage 127's net: a playing attack seen by a frame that polls the behaviour, while Re:CoM's
/// state (`now`, its index) is not the attack's own state (`attack_state`): its state was left
/// without its end (a Re:CoM state, stun, Stop, a Re:CoM flinch…, took over and handed back to
/// state 1), so it is released. A steered attack is polled in its own attack state: it is kept
/// there (0.69.0 to 0.75.2 released every steered attack on its first step: the Bandit's leap
/// restarted every other frame, owner 2026-11-02).
pub fn attack_state_left(a: Running, now: Option<usize>, attack_state: i32) -> bool {
    a.attack && a.playing && now != usize::try_from(attack_state).ok()
}

/// What a steered attack's state does after the behaviour ran in it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AfterPoll {
    /// The behaviour kept its request: the attack plays on.
    Keep,
    /// A new attack: its state is asked for again (after this state's end).
    NewAttack,
    /// A plain action (a jump start…): it carries on in state 1.
    Plain,
    /// Another motion replaced it: the attack ends.
    Replaced,
    /// No action is left.
    Gone,
}

/// `before`: the request the steered attack had before the behaviour ran; `now`: the running
/// action after; `current`: the request playing after. KH1's rows end with their request
/// (`0x1402B47A0`): the attack ends when the behaviour plays another motion or starts something
/// else.
pub fn after_poll(before: Option<u16>, now: Option<Running>, current: Option<u16>) -> AfterPoll {
    match now {
        Some(a) if a.attack && !a.playing => AfterPoll::NewAttack,
        Some(a) if a.steered && Some(a.request) == before && current == before => AfterPoll::Keep,
        Some(a) if !a.attack => AfterPoll::Plain,
        Some(_) => AfterPoll::Replaced,
        None => AfterPoll::Gone,
    }
}

/// Re:CoM's own super armour, `enemy+0x260` bits 20 and 21 (M11, the Pirate's `sys 42`,
/// [`crate::enemy::Behaviour::armoured`]). Re:CoM's hit handler `0x14017F7E0` applies the damage, then
/// its reaction picker `0x14017D260`: with bit 20 the blow-away and knock kinds (2..6, 0x12, 0x1D,
/// 0x1E) take the flinch's path `0x14017D800` as kind 1 does, and with bit 21 that path only plays the
/// damage voice (`0x140181B50` / `0x140181A50`) and asks for no state (`+0x68` untouched). The status
/// kinds keep their states (Stop 8 / 0x17, stun 7 with its gauge, Gravity's flatten 0xD, freeze 0x11,
/// 0xB / 0xC / 0xE..0x10 / 0x14 / 0x16 / 0x1F), as decision M10 keeps them; a killing hit still goes
/// to the defeat (`0x140180A80`). Checked (decomp).
pub const RECOM_ARMOUR: u64 = 0x30_0000;

/// The flags for the hit handler's call with the armour on.
pub fn armour_flags(flags: u64) -> u64 {
    flags | RECOM_ARMOUR
}

/// The flags after the call: the two bits as they were before (Re:CoM's own, if it set them).
pub fn restore_armour(now: u64, before: u64) -> u64 {
    (now & !RECOM_ARMOUR) | (before & RECOM_ARMOUR)
}

/// Whether the armoured call took a damaging hit without a reaction: the handler answered 1 (a hit
/// applied), the enemy lives, and no state was asked for (`+0x68` the same as before the call).
pub fn armour_took(result: u64, hp: i32, next_before: usize, next_after: usize) -> bool {
    result == 1 && hp > 0 && next_before == next_after
}

/// KH1's clash kinds (6 / 7, `0x1402B8E10`'s cases 6 / 7): the fliers' handlers keep their reaction
/// (`hp[0xC] & 0x1F` tested against 6 and 7 before zeroing it; Checked (files): `ex_2210.bd` `0x1F0C`,
/// `ex_2320.bd` `0x2484`, `ex_2280.bd` `0x1FBC`).
pub fn is_clash(kind: u8) -> bool {
    matches!(kind, 6 | 7)
}

/// What Re:CoM's hit handler `0x14017F7E0` is about to take, as its gather `0x140154EB0` picks it: the
/// enemy's hurt bodies in order, the first with the repel bit (`+0x9C & 8`) or the guarded bit (`& 0x20`)
/// (the handler's guard branch: no damage, no reaction picker, returns 0) or a hit count (`+0x90` ≠ 0, a
/// damaging hit: the hit apply `0x14016F1C0` writes it with the attack's Re:CoM kind into `+0x80` and its
/// owner into `+0x98`, before the handler runs). Checked (decomp).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gathered {
    /// No hit waits (the handler returns 0 at once).
    Nothing,
    /// A guarded or repelled hit, at this body.
    Guarded(usize),
    /// A damaging hit, at this body.
    Hit(usize),
}

/// [`Gathered`] from the bodies' `(+0x9C flags, +0x90 count)` in order.
pub fn gather(bodies: impl IntoIterator<Item = (u32, i16)>) -> Gathered {
    for (i, (flags, count)) in bodies.into_iter().enumerate() {
        if flags & 0x28 != 0 {
            return Gathered::Guarded(i);
        }
        if count != 0 {
            return Gathered::Hit(i);
        }
    }
    Gathered::Nothing
}

/// A model of the host for tests (the behaviours', and `kh1-recom`'s with each class's map and
/// manifest; the DLL never runs it): Re:CoM's state 1 and attack states as
/// `win::kh1_enemy` runs them (`think_step`, `attack_begin`, `attack_step`, `tick`), on a flat
/// floor at y = 0, through [`attack_state_left`] and [`after_poll`]. Not modelled: reactions,
/// Re:CoM's own states, hits. Re:CoM's state machine (`0x140242F50`, the enemy's `+0x58`): a state
/// asked for (`+0x68`) begins at the next update, the old state's end, then the new one's begin and,
/// unless the begin asked for another, its step in the same update (Checked (decomp)).
pub mod model {
    use super::*;
    use crate::enemy::{react::req, Behaviour, Cmd, Engine, World};
    use crate::math::Vec3;

    pub struct Model<C: Engine> {
        pub ai: Box<dyn Behaviour>,
        pub c: C,
        /// KH1 motion lengths by request.
        pub frames: Vec<(u16, f32)>,
        /// Requests whose clip loops in Re:CoM (the asset step's `loop`).
        pub loops: Vec<u16>,
        /// The class's Re:CoM attack state by attack index (`kh1_enemy::Class::attack_states`).
        pub attack_states: Vec<i32>,
        pub pos: Vec3,
        pub moved: f32,
        /// Re:CoM's state (1, or an attack state) and the one asked for.
        pub state: i32,
        pub next: Option<i32>,
        /// The running action and its attack index.
        pub action: Option<(Running, Option<u8>)>,
        pub current: Option<u16>,
        /// Frames the current request has played.
        pub age: f32,
        pub queued: Option<u16>,
        /// Attacks released by Stage 127's net.
        pub released: usize,
        /// Every command, with its frame.
        pub log: Vec<(usize, Cmd<u8>)>,
        /// Per frame: Re:CoM's state and the request playing at the frame's end.
        pub trace: Vec<(i32, Option<u16>)>,
        pub frame: usize,
    }

    impl<C: Engine> Model<C> {
        pub fn new(ai: Box<dyn Behaviour>, c: C, frames: Vec<(u16, f32)>, loops: Vec<u16>, attack_states: Vec<i32>) -> Self {
            Model { ai, c, frames, loops, attack_states, pos: Vec3::ZERO, moved: 0.0, state: 1, next: None, action: None, current: None, age: 0.0, queued: None, released: 0, log: Vec::new(), trace: Vec::new(), frame: 0 }
        }

        fn len(&self, r: u16) -> f32 {
            self.frames.iter().find(|x| x.0 == r).map_or(30.0, |x| x.1)
        }

        fn ended(&self) -> bool {
            self.current.is_none_or(|r| self.age >= self.len(r))
        }

        fn play(&mut self, r: u16) {
            self.current = Some(r);
            self.age = 0.0;
        }

        fn attack_state(&self, i: Option<u8>) -> i32 {
            self.attack_states.get(i.unwrap_or(0) as usize).copied().unwrap_or(2)
        }

        /// `n` frames, Sora at `sora`.
        pub fn run(&mut self, sora: Vec3, n: usize) {
            for _ in 0..n {
                if let Some(k) = self.next.take() {
                    self.state = k;
                    if k != 1 {
                        self.attack_begin();
                    }
                }
                if self.next.is_some() {
                    // The begin asked for another state: no step.
                } else if self.state == 1 {
                    self.tick(sora, true);
                } else {
                    self.attack_step(sora);
                }
                self.trace.push((self.state, self.current));
                self.frame += 1;
            }
        }

        fn attack_begin(&mut self) {
            match self.action {
                Some((a, i)) if a.attack => {
                    self.play(a.request);
                    self.action = Some((Running { playing: true, ..a }, i));
                    if a.steered {
                        self.queued = None;
                    }
                }
                // Re:CoM's own begin (never expected).
                _ => self.next = Some(1),
            }
        }

        fn attack_step(&mut self, sora: Vec3) {
            let steered = self.action.is_some_and(|(a, _)| a.steered && a.playing);
            let mut end = if steered {
                let looped = self.current.is_some_and(|r| self.loops.contains(&r));
                !looped && self.ended()
            } else {
                self.action.is_some() && self.ended()
            };
            if end {
                self.action = None;
            }
            let poll = steered && !end;
            let before = self.action.map(|(a, _)| a.request);
            self.tick(sora, poll);
            if poll {
                match after_poll(before, self.action.map(|a| a.0), self.current) {
                    AfterPoll::Keep => {}
                    AfterPoll::Replaced => {
                        self.action = None;
                        end = true;
                    }
                    _ => end = true,
                }
            }
            if end {
                // Re:CoM's attack end (the default state), then a new attack's state again.
                let again = self.action.filter(|(a, _)| a.attack && !a.playing).map(|(_, i)| self.attack_state(i));
                self.next = Some(again.unwrap_or(1));
            }
        }

        fn tick(&mut self, sora: Vec3, poll: bool) {
            self.age += 1.0;
            let now = usize::try_from(self.state).ok();
            if let Some((a, i)) = self.action {
                if poll && attack_state_left(a, now, self.attack_state(i)) {
                    self.released += 1;
                    self.action = None;
                    self.queued = None;
                    self.play(req::IDLE);
                }
            }
            if self.ended() {
                if let Some((a, _)) = self.action.filter(|(a, _)| a.playing && !a.attack) {
                    if self.current == Some(a.request) {
                        self.action = None;
                    }
                }
                if let Some(q) = self.queued.take() {
                    self.play(q);
                }
            }
            if poll && self.action.is_none_or(|(a, _)| a.steered) {
                let (airborne, flying) = {
                    let b = self.ai.body();
                    (b.airborne(), b.flying)
                };
                // The battle floor (Sora standing at y = 0), as the DLL passes it.
                self.ai.set_floor(0.0);
                let contact = self.pos.y >= 0.0;
                let w = World { step: 1.0, pos: self.pos, target: Some(sora), grounded: contact && !airborne && !flying, airborne, moved: self.moved, motion: self.current };
                for c in self.ai.frame(w) {
                    self.apply(&c);
                    self.log.push((self.frame, c));
                }
            }
            let steer = self.ai.take_steer();
            let d = self.ai.body().step(1.0, self.pos.y, steer, &self.c);
            self.pos += d;
            self.moved = d.length();
            if self.pos.y >= 0.0 && self.ai.body().falling() {
                self.pos.y = 0.0;
                self.ai.body().land();
            }
            // A body on KH1's ground state above the floor (the Neoshadow's `sys 88` at the end of its dive, M11)
            // falls with Re:CoM's mover gravity, here at KH1's fall cap a frame.
            let (airborne, flying) = {
                let b = self.ai.body();
                (b.airborne(), b.flying)
            };
            if !airborne && !flying && self.pos.y < 0.0 {
                self.pos.y = (self.pos.y + 20.0).min(0.0);
            }
        }

        fn apply(&mut self, c: &Cmd<u8>) {
            match *c {
                Cmd::Motion { request, .. } | Cmd::MotionAt { request, .. } => {
                    if self.current != Some(request) {
                        self.queued = None;
                        self.play(request);
                    }
                }
                // `sys 13` of the playing request: it starts again (M11, the Fat Bandit).
                Cmd::Restart { request, start, .. } => {
                    self.queued = None;
                    self.play(request);
                    self.age = start;
                }
                Cmd::Queue { request, .. } => self.queued = Some(request),
                Cmd::Action { request, attack, then, .. } => {
                    let a = Running { request, attack: attack.is_some(), steered: false, playing: attack.is_none() };
                    self.action = Some((a, attack));
                    match attack {
                        Some(_) => self.next = Some(self.attack_state(attack)),
                        None => {
                            self.play(request);
                            self.queued = Some(then);
                        }
                    }
                }
                Cmd::Attack { request, attack, .. } => {
                    self.action = Some((Running { request, attack: true, steered: true, playing: false }, Some(attack)));
                    self.next = Some(self.attack_state(Some(attack)));
                }
                _ => {}
            }
        }

        /// A damaging hit of KH1 kind `kind` arrives now, as `win::kh1_enemy`'s `armour_on` / `armour_off`
        /// hand it to the behaviour (no launch yet: base 0): true when the behaviour took it without a
        /// reaction (Re:CoM's super armour: Re:CoM's state stays, the action goes on). False: Re:CoM's
        /// flinch would follow (not modelled).
        pub fn armoured_hit(&mut self, kind: u8, dir: Vec3) -> bool {
            if !self.ai.armoured(self.current, 0, Some(kind)) {
                return false;
            }
            self.ai.set_hit_kind(kind);
            self.ai.on_armoured_hit(dir);
            true
        }

        /// The frames on which request `r` was asked for as a steered attack.
        pub fn attacks_of(&self, r: u16) -> Vec<usize> {
            self.log.iter().filter_map(|(f, c)| matches!(c, Cmd::Attack { request, .. } if *request == r).then_some(*f)).collect()
        }

        /// The longest run of frames `r` played without a restart, from the trace.
        pub fn longest_run(&self, r: u16) -> usize {
            let (mut best, mut run) = (0, 0);
            for &(_, c) in &self.trace {
                run = if c == Some(r) { run + 1 } else { 0 };
                best = best.max(run);
            }
            best
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rules() {
        let a = Running { request: 217, attack: true, steered: true, playing: true };
        // Polled in its own attack state: kept; in state 1 (a Re:CoM state handed back): left.
        assert!(!attack_state_left(a, Some(3), 3));
        assert!(attack_state_left(a, Some(1), 3));
        assert!(!attack_state_left(Running { playing: false, ..a }, Some(1), 3));
        assert!(!attack_state_left(Running { attack: false, ..a }, Some(1), 3));
        assert_eq!(after_poll(Some(217), Some(a), Some(217)), AfterPoll::Keep);
        assert_eq!(after_poll(Some(217), Some(a), Some(218)), AfterPoll::Replaced);
        assert_eq!(after_poll(Some(211), Some(Running { request: 212, playing: false, ..a }), Some(211)), AfterPoll::NewAttack);
        assert_eq!(after_poll(Some(217), Some(Running { attack: false, steered: false, ..a }), Some(5)), AfterPoll::Plain);
        assert_eq!(after_poll(Some(217), None, Some(0)), AfterPoll::Gone);
    }

    /// Re:CoM's super armour for one hit handler call: bits 20 / 21 on, then put back as they were;
    /// the hit counts as armoured only when it was applied to a living enemy and no state was asked.
    #[test]
    fn the_armour_rules() {
        assert_eq!(armour_flags(0x1), 0x30_0001);
        assert_eq!(restore_armour(0x30_0001, 0x1), 0x1);
        assert_eq!(restore_armour(0x30_0000, 0x20_0000), 0x20_0000, "Re:CoM's own bit 21 stays");
        assert_eq!(restore_armour(0x30_0000 | 0x2_0000_0000, 0), 0x2_0000_0000, "other bits the handler wrote stay");
        assert!(armour_took(1, 10, 0x780, 0x780));
        assert!(!armour_took(0, 10, 0x780, 0x780), "no hit (dropped, guarded)");
        assert!(!armour_took(1, 0, 0x780, 0x780), "the killing hit");
        assert!(!armour_took(1, 10, 0, 0x940), "a Re:CoM state (Stop) was asked for");
    }

    /// The gather's order (`0x140154EB0`): the first body with a guard / repel bit or a hit count wins.
    #[test]
    fn the_gather_rule() {
        assert_eq!(gather([]), Gathered::Nothing);
        assert_eq!(gather([(0, 0), (0, 0)]), Gathered::Nothing);
        assert_eq!(gather([(0, 0), (0, 1)]), Gathered::Hit(1));
        assert_eq!(gather([(0x20, 0), (0, 1)]), Gathered::Guarded(0));
        assert_eq!(gather([(0, 1), (0x8, 0)]), Gathered::Hit(0));
        assert_eq!(gather([(0x1, 0), (0x8, 1)]), Gathered::Guarded(1), "the guard bit (body flag 1) alone is not a hit");
        assert!(is_clash(6) && is_clash(7) && !is_clash(5) && !is_clash(0));
    }
}
