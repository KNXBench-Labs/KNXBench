← Previous: [What is KNXBench?](01-what-is-knxbench.md) · [Manual index](../README.md)

# Why KNXBench exists

The short version: the author switched to Linux, then went looking for a KNX
tool that felt native there. He did not find one. ETS itself is a mature,
capable product — it also happens to be a Windows program, and running it
comfortably on Linux meant a virtual machine, a Wine layer, or a level of
compromise that gets old fast when you just want to look at a group address.

Faced with a gap and a stubborn refusal to boot Windows, he did the only
reasonable thing: he started an AI assistant and began building the tool
himself. Several hundred euros and a few million tokens later, KNXBench had
its first working alpha. In hindsight: I must have been drunk. That
questionable decision is now this project.

> **Note**
>
> If you are reading this manual, the decision has already been made. What
> follows is the part where it gets serious.

## What KNXBench actually optimizes for

Once the joke settles, the project runs on a fixed set of priorities, in this
order:

**Correctness → Data integrity → Compatibility → Maintainability → UX → Performance**

In practice that means: an import that cannot fully understand something
never throws that something away — it preserves it, or reports it as
unsupported. A feature that looks convenient but risks silently corrupting a
project loses to the boring, correct option every time. Compatibility with
ETS project files matters, but only as far as it can be verified against real
test material; a shinier UI does not outrank a validated import path, and raw
speed does not outrank either.

## An independent implementation, not a copy

KNXBench does not imitate ETS internals, and it does not reverse-engineer
proprietary formats it cannot document. It has its own domain model, its own
`.knxproj` parser, its own native storage format, and its own product
database and KNXnet/IP stack. Where ETS project files are read or written,
that support is built against documented behavior and real files, not against
assumptions about how ETS happens to work internally.

## Linux-first, on purpose

KNXBench is built for Linux as the primary target, not as an afterthought
ported in later. The desktop shell, the packaging, and the day-to-day
development all happen on Linux first. That focus is also why the tested
boundary described in [Linux setup](05-linux-setup.md) is currently narrow —
better to be honest about what has actually been tested than to promise
portability nobody has checked.

[Manual index](../README.md) · Next: [Project status](03-project-status.md) →
