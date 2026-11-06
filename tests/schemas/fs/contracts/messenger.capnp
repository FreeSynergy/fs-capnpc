# fs-messenger contract — DRAFT 0.1 (2026-11-05)
# Source of truth for the frontend <-> program interface (ADR 0021). JSON, CLI and MCP are generated
# from this file. Capability ids and groups: docs/en/programs/apps/messenger/requirements.md.
# Evolution rules (Cap'n Proto): never renumber or remove fields/methods; only append.
# Compiled successfully 2026-11-05 together with common.capnp (capnpc-embedded 0.2.0 / capnp 1.4.0 + capnpc 0.23).

@0xd6c1b5e3a2f40917;   # file id (generate a fresh one with `capnp id` before first release)


using C = import "common.capnp";
using Uuid = C.Uuid;
using Timestamp = C.Timestamp;
using Cursor = C.Cursor;
using Problem = C.Problem;
using Envelope = C.Envelope;
using Blob = C.Blob;

# pages: one struct per item type (Cap'n Proto has no List(T) over generic parameters)
struct MessagePage      { items @0 :List(MessageSummary);      next @1 :Cursor; }
struct ConversationPage { items @0 :List(ConversationSummary); next @1 :Cursor; }

enum Encryption { mls @0; openpgp @1; megolm @2; transportOnly @3; none @4; }   # ADR 0018

enum Kind { mail @0; chat @1; comment @2; }

enum DeliveryState { sending @0; sent @1; delivered @2; read @3; failed @4; }  # stored locally (ADR 0037)

struct Attachment {
  name      @0 :Text;
  mediaType @1 :Text;
  size      @2 :UInt64;
  blob      @3 :Blob;                                       # capability to fetch the bytes
}

struct Reaction { key @0 :Text; count @1 :UInt32; mine @2 :Bool; }

struct MessageSummary {
  envelope   @0 :Envelope;
  kind       @1 :Kind;
  encryption @2 :Encryption;
  state      @3 :DeliveryState;
  unread     @4 :Bool;
  flagged    @5 :Bool;
  preview    @6 :Text;                                      # plain-text snippet
}

struct Message {
  summary     @0 :MessageSummary;
  bodyText    @1 :Text;                                     # normalised text
  bodyHtml    @2 :Text;                                     # sanitised, no remote loading (html-view)
  attachments @3 :List(Attachment);
  reactions   @4 :List(Reaction);
  original    @5 :Blob;                                     # original as received (ADR 0030)
  originalMediaType @6 :Text;                               # message/rfc822, MIMI content, Matrix event
}

struct ConversationSummary {
  id          @0 :Uuid;
  name        @1 :Text;
  kind        @2 :Kind;
  encryption  @3 :Encryption;
  unreadCount @4 :UInt32;
  lastMessage @5 :MessageSummary;
  pinned      @6 :Bool;
  folders     @7 :List(Text);                               # folder = group = distribution list (ADR 0037)
}

# ------------------------------------------------------------ root ----

interface Messenger {
  # what this program offers (capability ids like "message.send"); frontends use what they need
  capabilities  @0 () -> (ids :List(Text), contractVersion :Text);

  accounts      @1 () -> (accounts :Accounts);
  conversations @2 () -> (conversations :Conversations);
  compose       @3 () -> (compose :Compose);
  search        @4 () -> (search :Search);
  rules         @5 () -> (rules :Rules);                    # optional capability: may be null
  subscribe     @6 (filter :EventFilter, listener :Listener) -> (subscription :Subscription);
}

interface Accounts {
  list   @0 () -> (accounts :List(Account));
  add    @1 (setup :AccountSetup) -> (account :Account, problem :Problem);
  remove @2 (id :Uuid) -> ();
}
struct Account      { id @0 :Uuid; address @1 :Text; kind @2 :Text; encryptionReady @3 :Bool; }
struct AccountSetup { address @0 :Text; kind @1 :Text; secret @2 :Text; }  # kind: "mail", "matrix", "mimi"

interface Conversations {
  list    @0 (folder :Text, onlyUnread :Bool, cursor :Cursor) -> (page :ConversationPage);
  open    @1 (id :Uuid) -> (conversation :Conversation);
  nextUnread @2 (after :Uuid) -> (message :MessageSummary);  # frontend no. 1 (swipe)
}

interface Conversation {
  messages @0 (cursor :Cursor) -> (page :MessagePage);
  message  @1 (id :Uuid) -> (message :Message);
  markRead @2 (upTo :Uuid) -> ();                            # "read up to X"
  pin      @3 (pinned :Bool) -> ();
  move     @4 (folder :Text) -> ();
  act      @5 (id :Uuid, action :MessageAction) -> (problem :Problem);
}

struct MessageAction {
  union {
    delete   @0 :Void;
    flag     @1 :Bool;
    unread   @2 :Bool;
    react    @3 :Text;                                       # reaction key; empty = undo
    forward  @4 :List(Text);                                 # recipients
    edit     @5 :Text;                                       # new text; first version's id stays
  }
}

interface Compose {
  draft  @0 (replyTo :Uuid) -> (draft :Draft);              # empty replyTo = new message
}

interface Draft {
  setRecipients @0 (to :List(Text), cc :List(Text), bcc :List(Text)) -> (unreachable :List(Text));
  setText       @1 (text :Text, mediaType :Text) -> ();     # text/plain or text/markdown (simple editor)
  attach        @2 (name :Text, mediaType :Text, data :Data) -> ();
  encryptionPlan @3 () -> (encryption :Encryption, missingKeys :List(Text));  # ADR 0018: find a way
  save          @4 () -> (id :Uuid);
  send          @5 (at :Timestamp) -> (id :Uuid, problem :Problem);  # at = 0 now, else schedule send
  discard       @6 () -> ();
}

interface Search {
  query @0 (text :Text, filter :SearchFilter, cursor :Cursor) -> (page :MessagePage);
}
struct SearchFilter { from @0 :Text; unread @1 :Bool; hasAttachment @2 :Bool; since @3 :Timestamp; }

interface Rules {
  list @0 () -> (sieve :Text);                               # rules stored as Sieve
  set  @1 (sieve :Text) -> (problem :Problem);
}

# ------------------------------------------------------------ events ----

struct EventFilter { conversations @0 :List(Uuid); kinds @1 :List(Kind); }

struct Event {
  union {
    created  @0 :MessageSummary;
    updated  @1 :MessageSummary;
    deleted  @2 :Uuid;
    state    @3 :StateChange;
  }
}
struct StateChange { message @0 :Uuid; state @1 :DeliveryState; }

interface Listener     { event @0 (event :Event) -> stream; }
interface Subscription { }                                    # dropping it ends the subscription
