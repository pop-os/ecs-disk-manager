use ecs_disk_manager_dbus::*;

use dbus::{
    arg::Arg,
    tree::{Factory, MTFn, Method, MethodErr, Signal},
    Message, MessageItem,
};

pub struct DbusFactory<'a> {
    factory: &'a Factory<MTFn<()>, ()>,
}

impl<'a> DbusFactory<'a> {
    pub fn new(factory: &'a Factory<MTFn<()>, ()>) -> Self { DbusFactory { factory } }

    pub fn method<I, E>(&self, name: &'static str, ins: I) -> MethodInstance
    where
        I: Fn(&Message) -> Result<Vec<MessageItem>, E> + 'static,
        E: ::std::error::Error,
    {
        let method = self.factory.method(name, (), move |m| match ins(m.msg) {
            Ok(messages) => {
                let mut mret = m.msg.method_return();
                for message in messages {
                    mret = mret.append(message);
                }

                Ok(vec![mret])
            }
            Err(why) => {
                let mut error_message = format!("{}", why);
                let mut cause = why.source();
                while let Some(source) = cause {
                    error_message.push_str(&format!(": {}", source));
                    cause = source.source();
                }
                error!("{}", error_message);
                Err(MethodErr::failed(&error_message))
            }
        });

        MethodInstance(method)
    }

    pub fn signal(&self, name: &'static str) -> SignalInstance {
        SignalInstance(self.factory.signal(name, ()))
    }
}

pub struct MethodInstance(Method<MTFn<()>, ()>);

impl MethodInstance {
    pub fn inarg<T: Arg>(self, s: &str) -> Self { MethodInstance(self.0.inarg::<T, _>(s)) }

    pub fn outarg<T: Arg>(self, s: &str) -> Self { MethodInstance(self.0.outarg::<T, _>(s)) }

    pub fn consume(self) -> Method<MTFn<()>, ()> { self.0 }
}

pub struct SignalInstance(Signal<()>);

impl SignalInstance {
    pub fn sarg<T: Arg>(self, name: &str) -> Self { SignalInstance(self.0.sarg::<T, _>(name)) }

    pub fn consume(self) -> Signal<()> { self.0 }
}
