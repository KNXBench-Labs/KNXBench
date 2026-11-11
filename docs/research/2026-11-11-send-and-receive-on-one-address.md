# Send and receive on one group address

Question (WP-39, found in the review of issue #47): a project may hold a send
and a receive link from one communication object to the same group address
("Send + Receive"). What does that mean for the device, and how do ETS and the
KNX specification represent it?

## Sources inspected

Local copy of The KNX Standard v3.0.0 (licensed; paraphrased here, page
numbers are the printed footers):

- `03_05_01 Resources v01.10.01 AS`, §4.17.3.3.1 *Frame transmission* (p. 246)
  and §4.17.3.3.2 *Frame reception* with its footnote (p. 247): Group Object
  Association Table realisation type 1 (System 1, masks 0010h–0012h).
- Same document, §4.17.9.4.1 *Frame transmission* (p. 258), §4.17.9.4.2
  *Frame reception* and §4.17.9.5 *Usage by the Management Client* (p. 259):
  GrOAT Easy 3, used by mask 0701h.
- Same document, §4.17.5 (pp. 248–251): realisation type 6 (System B / 300),
  "first matching entry" for transmission (p. 250/251).
- `03_03_07 Application Layer v02.01.01 AS`, §3.1.1 *The relation between
  TSAPs and ASAPs* (p. 12).

KNX Association, Project Schema23 v01.00.00, `ComObjectInstanceRef/@Links`:
the first group address in the list is the sending one (already the basis of
the ETS import, `crates/knx-etsproj/src/map.rs`).

## Findings

**[V]** Reception does not depend on the sending marker. On a received group
telegram the device searches the whole association table and indicates every
association whose group address matches, the sending association included
(Resources §4.17.3.3.2 and §4.17.9.4.2; Application Layer §3.1.1). The footnote
on p. 247 states the consequence for a repeated entry: one telegram yields as
many indications as the address occurs for that object.

**[V]** Transmission uses exactly one association per object: the association
numbered like the object in realisation type 1, the first matching entry in
Easy 3 and type 6. An object with a sending address therefore both sends and
receives on it.

**[V]** The project format marks the first group address of an object's
`@Links` as the sending one and has no separate "send and receive" state for
one address. **[I]** It does not say whether an id may repeat in that list;
KNXBench has not seen a repeated id in imported ETS projects, so ETS is taken to
list an address once per object.

**[I]** "Send + Receive" on one address is, for the device, exactly one sending
association. Writing two entries for it is at best redundant (double
indication on legacy tables) and is refused by KNXBench's Easy 3 and System B
table builders as a duplicate link.

## Decision

The download request (`knx-productdb` `image_request::links`) merges the links
of one object by group address: a send and a receive link to one address
become one sending association, two links that reach one address become one
association. The project keeps both links unchanged; only the device tables
follow the specification. Two *different* sending addresses on one object stay
refused (`SeveralSendingLinks`).
