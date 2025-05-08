use std::pin::Pin;
use std::sync::{ Mutex, LazyLock };
use argh::FromArgs;
use autocxx::moveit::moveit;
use crate::sys::lldb;
use crate::util::u64ptr;

/// MyDbg Trace command
#[derive(FromArgs)]
pub struct Command {
    /// trace start
    #[argh(switch)]
    start: bool,

    /// record regsiter
    #[argh(option, short = 'r')]
    record: Option<String>,

    /// end address
    #[argh(option, short = 'e')]
    done: Option<String>,
}

#[derive(Default)]
struct Status {
    records: Vec<u64>,
    done: Vec<u64>,

    logs: Vec<(u32, u64)>,
}

static STATUS: LazyLock<Mutex<Status>> = LazyLock::new(Default::default);

impl Command {
    pub fn execute(self, mut debugger: Pin<&mut lldb::SBDebugger>) -> anyhow::Result<()> {
        let mut status = STATUS.lock().unwrap();
        let status = &mut *status;

        moveit!{
            let mut target = debugger.as_mut().GetSelectedTarget();
            let mut process = target.as_mut().GetProcess();
        }

        if let Some(addr) = self.record.as_ref() {
            let addr = u64ptr(&addr)?;
            status.records.push(addr);
        }
        
        if let Some(addr) = self.done.as_ref() {
            let addr = u64ptr(&addr)?;
            status.done.push(addr);
        }

        if !self.start {
            return Ok(());
        }

        match process.as_mut().GetState() {
            lldb::StateType::eStateStopped => (),
            state => anyhow::bail!("bad state: {:?}", state as u32)
        }

        if status.done.is_empty() {
            anyhow::bail!("need exit point");
        }

        moveit!{
            let mut broadcast = process.as_ref().GetBroadcaster();
            let mut listener = debugger.as_mut().GetListener();
            let mut event = lldb::SBEvent::new();
        }

        listener.as_mut().StartListeningForEvents(&broadcast, 1 << 0);

        moveit!{
            let mut err = process.as_mut().Continue();
        }

        let _err = err;

        if !listener.as_mut().WaitForEvent(10, event.as_mut()) {
            println!("false false");
        }

        println!("wait wait");
      
        Ok(())        
    }
}
