//! Pure mobile connection lifecycle. No network, credentials, OS calls or engine
//! execution. A platform backend must supply real permission and health events.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Engine { VpCore, SingBox, Mihomo, Xray, Shoes, Meow }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase { Idle, AwaitingPermission, Starting, Verifying, Connected, Stopping, StopUncertain }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error { Busy, UnsupportedEngine, InvalidTransition, StaleEvent, CounterExhausted }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect { RequestPermission, StartEngine, VerifyTraffic, StopEngine, None }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ticket(u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HealthEvidence { pub protected_sockets: bool, pub packet_forwarding: bool, pub dns_verified: bool }

/// Caller must derive availability from the actual platform adapter, not from
/// a config compiler or the mere presence of an engine name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Availability { Ready, Unavailable }

#[derive(Debug)]
pub struct Connection { phase: Phase, generation: u64, engine: Option<Engine> }
impl Default for Connection {
    fn default() -> Self { Self { phase: Phase::Idle, generation: 0, engine: None } }
}
impl Connection {
    pub fn phase(&self) -> Phase { self.phase }
    pub fn engine(&self) -> Option<Engine> { self.engine }
    fn ticket(&self) -> Ticket { Ticket(self.generation) }
    fn current(&self, ticket: Ticket) -> Result<(), Error> {
        if ticket == self.ticket() { Ok(()) } else { Err(Error::StaleEvent) }
    }
    fn next(&mut self) -> Result<Ticket, Error> {
        self.generation = self.generation.checked_add(1).ok_or(Error::CounterExhausted)?;
        Ok(self.ticket())
    }
    pub fn start(&mut self, engine: Engine, availability: Availability) -> Result<(Ticket, Effect), Error> {
        if self.phase != Phase::Idle { return Err(Error::Busy); }
        if availability != Availability::Ready { return Err(Error::UnsupportedEngine); }
        let ticket = self.next()?;
        self.engine = Some(engine); self.phase = Phase::AwaitingPermission;
        Ok((ticket, Effect::RequestPermission))
    }
    pub fn permission(&mut self, ticket: Ticket, granted: bool) -> Result<Effect, Error> {
        self.current(ticket)?;
        if self.phase != Phase::AwaitingPermission { return Err(Error::InvalidTransition); }
        if granted { self.phase = Phase::Starting; Ok(Effect::StartEngine) }
        else { self.phase = Phase::Idle; self.engine = None; Ok(Effect::None) }
    }
    pub fn started(&mut self, ticket: Ticket) -> Result<Effect, Error> {
        self.current(ticket)?;
        if self.phase != Phase::Starting { return Err(Error::InvalidTransition); }
        self.phase = Phase::Verifying; Ok(Effect::VerifyTraffic)
    }
    pub fn verified(&mut self, ticket: Ticket, evidence: HealthEvidence) -> Result<Effect, Error> {
        self.current(ticket)?;
        if self.phase != Phase::Verifying { return Err(Error::InvalidTransition); }
        if evidence.protected_sockets && evidence.packet_forwarding && evidence.dns_verified {
            self.phase = Phase::Connected; Ok(Effect::None)
        } else {
            self.phase = Phase::Stopping; Ok(Effect::StopEngine)
        }
    }
    /// A fresh ticket invalidates every pending start/permission/health callback.
    /// Counter exhaustion leaves the prior state unchanged and requests no effect.
    pub fn stop(&mut self) -> Result<(Ticket, Effect), Error> {
        match self.phase {
            Phase::Idle => Ok((self.ticket(), Effect::None)),
            Phase::AwaitingPermission => { let t=self.next()?; self.phase=Phase::Idle; self.engine=None; Ok((t,Effect::None)) },
            Phase::Starting | Phase::Verifying | Phase::Connected => { let t=self.next()?; self.phase=Phase::Stopping; Ok((t,Effect::StopEngine)) },
            Phase::Stopping | Phase::StopUncertain => Err(Error::Busy),
        }
    }
    pub fn stopped(&mut self, ticket: Ticket, confirmed: bool) -> Result<Effect, Error> {
        self.current(ticket)?;
        if self.phase != Phase::Stopping { return Err(Error::InvalidTransition); }
        if confirmed { self.phase=Phase::Idle; self.engine=None; }
        else { self.phase=Phase::StopUncertain; }
        Ok(Effect::None)
    }
    /// Only a platform-owned reconciliation may report the exact engine and
    /// tunnel absent. An uncertain observation never re-enables connection.
    pub fn reconcile_stopped(&mut self, ticket: Ticket, absence_confirmed: bool) -> Result<(), Error> {
        self.current(ticket)?;
        if self.phase != Phase::StopUncertain || !absence_confirmed { return Err(Error::InvalidTransition); }
        self.phase=Phase::Idle; self.engine=None; Ok(())
    }
    /// Any failure after engine start must trigger cleanup; permission-stage
    /// failure can return to idle because no engine was requested.
    pub fn failed(&mut self, ticket: Ticket) -> Result<Effect, Error> {
        self.current(ticket)?;
        match self.phase {
            Phase::AwaitingPermission => { self.phase=Phase::Idle; self.engine=None; Ok(Effect::None) },
            Phase::Starting | Phase::Verifying | Phase::Connected => { self.phase=Phase::Stopping; Ok(Effect::StopEngine) },
            _ => Err(Error::InvalidTransition),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const OK: HealthEvidence=HealthEvidence{protected_sockets:true,packet_forwarding:true,dns_verified:true};
    #[test] fn unavailable_engine_never_falls_back_or_requests_permission() {
        let mut c=Connection::default();assert_eq!(c.start(Engine::Xray,Availability::Unavailable),Err(Error::UnsupportedEngine));assert_eq!(c.phase(),Phase::Idle);assert_eq!(c.engine(),None);
    }
    #[test] fn connected_requires_consent_start_and_full_evidence() {
        let mut c=Connection::default();let(t,e)=c.start(Engine::VpCore,Availability::Ready).unwrap();assert_eq!(e,Effect::RequestPermission);assert_eq!(c.verified(t,OK),Err(Error::InvalidTransition));assert_eq!(c.permission(t,true),Ok(Effect::StartEngine));assert_eq!(c.started(t),Ok(Effect::VerifyTraffic));assert_eq!(c.verified(t,OK),Ok(Effect::None));assert_eq!(c.phase(),Phase::Connected);
    }
    #[test] fn missing_health_requires_stop_and_never_claims_connected() {
        let mut c=Connection::default();let(t,_)=c.start(Engine::Meow,Availability::Ready).unwrap();c.permission(t,true).unwrap();c.started(t).unwrap();assert_eq!(c.verified(t,HealthEvidence{dns_verified:false,..OK}),Ok(Effect::StopEngine));assert_eq!(c.phase(),Phase::Stopping);c.stopped(t,true).unwrap();assert_eq!(c.phase(),Phase::Idle);
    }
    #[test] fn stale_callbacks_cannot_resurrect_a_stopped_connection() {
        let mut c=Connection::default();let(old,_)=c.start(Engine::SingBox,Availability::Ready).unwrap();c.permission(old,true).unwrap();let(stop,e)=c.stop().unwrap();assert_eq!(e,Effect::StopEngine);assert_eq!(c.started(old),Err(Error::StaleEvent));c.stopped(stop,true).unwrap();assert_eq!(c.phase(),Phase::Idle);
    }
    #[test] fn failed_cleanup_blocks_new_connection_until_explicit_reconciliation() {
        let mut c=Connection::default();let(t,_)=c.start(Engine::Shoes,Availability::Ready).unwrap();c.permission(t,true).unwrap();assert_eq!(c.failed(t),Ok(Effect::StopEngine));c.stopped(t,false).unwrap();assert_eq!(c.phase(),Phase::StopUncertain);assert_eq!(c.start(Engine::Mihomo,Availability::Ready),Err(Error::Busy));
    }
    #[test] fn uncertain_stop_requires_exact_absence_reconciliation() {
        let mut c=Connection::default();let(t,_)=c.start(Engine::VpCore,Availability::Ready).unwrap();c.permission(t,true).unwrap();c.failed(t).unwrap();c.stopped(t,false).unwrap();
        assert_eq!(c.reconcile_stopped(t,false),Err(Error::InvalidTransition));assert_eq!(c.phase(),Phase::StopUncertain);
        c.reconcile_stopped(t,true).unwrap();assert_eq!(c.phase(),Phase::Idle);
    }
    #[test] fn exhausted_generation_never_starts_an_engine() {
        let mut c=Connection{phase:Phase::Idle,generation:u64::MAX,engine:None};assert_eq!(c.start(Engine::VpCore,Availability::Ready),Err(Error::CounterExhausted));assert_eq!(c.phase(),Phase::Idle);
    }
    #[test] fn denied_permission_and_cancel_do_not_start_engine() {
        let mut c=Connection::default();let(t,_)=c.start(Engine::Mihomo,Availability::Ready).unwrap();assert_eq!(c.permission(t,false),Ok(Effect::None));let(old,_)=c.start(Engine::Xray,Availability::Ready).unwrap();let(_,e)=c.stop().unwrap();assert_eq!(e,Effect::None);assert_eq!(c.permission(old,true),Err(Error::StaleEvent));
    }
}
