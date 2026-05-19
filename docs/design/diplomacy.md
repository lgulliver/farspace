# FARSPACE Diplomacy System v3

Diplomacy v3 adds deterministic treaties, diplomatic communications, first-contact introductions, and AI diplomatic actions.

## Relationship states

- Unknown
- Contacted
- Neutral
- Cooperative
- Tense
- Hostile
- War

`GameState` stores both coarse status (`diplomacy`) and rich relationship data (`diplomacy_relationships`):

- `relationship_score`
- `tension_score`
- `trust_score`
- `last_major_diplomatic_event_turn`
- `active_treaties`
- `recent_grievances`
- `known_doctrine`
- `first_contact_turn`

## Relationship and tension rules

- First contact creates status from faction diplomacy profile.
- Relationship drift remains deterministic and doctrine-aware.
- Border pressure and doctrine weights influence escalation/calming pace.
- Relationship updates emit `RelationshipStateChanged`.
- Status remains inspectable; no opaque random behavior.

## Treaty types

Supported in v3:

- Non-Aggression Pact
- Truce (peace treaty)

Behavior:

- Non-Aggression Pact blocks war declaration while active.
- Truce ends war and blocks immediate redeclaration.
- Treaties have deterministic fixed durations.
- Expiry and cancellation emit events.

## Communication system

`GameState.diplomacy_pending_communications` stores pending messages with deterministic IDs.

Communication types:

- FirstContact
- TreatyProposal
- TreatyAccepted
- TreatyRejected
- Warning
- TributeDemand
- PeaceOffer
- WarDeclaration

Message fields:

- `communication_id`
- sender/receiver
- turn
- type
- tone
- title/body
- available responses
- optional expiry
- optional treaty type

Duplicate-spam prevention:

- Same sender/receiver/type/treaty pending message is not duplicated before expiry.

## Communication tones

Supported tones:

- Cooperative
- Formal
- Suspicious
- Threatening
- Hostile
- Desperate
- Triumphant

Tone derives deterministically from doctrine, relationship state, and communication type.

## AI diplomacy rules

Each turn, AI can deterministically:

- Propose non-aggression pact (cooperative/economic/science-oriented doctrines)
- Issue warnings (tense/hostile pressure posture)
- Demand tribute (high imperial/militarist hostility)
- Offer peace when losing wars
- Declare war under severe pressure and aggressive doctrine

No hidden randomness. Candidate and state ordering remain deterministic.

## War / peace / truce behavior

- `DeclareWar` now emits `WarDeclared` and creates war-declaration communication.
- War declaration is blocked by active Non-Aggression Pact or Truce.
- Peace acceptance emits `PeaceSigned`, sets relationship to Neutral, and starts Truce treaty.

## Public information rules

- Unknown empires stay hidden in diplomacy UI and dispatch summaries.
- Dispatch uses known-contact checks before naming empires for diplomacy/war items.
- Abstract wording used where visibility is restricted.

## Turn Report and Galactic Dispatch integration

Turn report now counts diplomacy outcomes:

- treaties
- wars
- peaces

Galactic Dispatch now surfaces major diplomacy events:

- first contact
- treaty signed/expired/cancelled
- warning issued
- tribute demanded
- war declared
- peace signed

## TUI diplomacy updates

Diplomacy screen now shows:

- selected empire
- relationship state
- relationship/tension/trust metrics (when known)
- active treaties
- keyboard actions for war/peace/treaty/warning/tribute/greeting

Communication modal supports:

- queued messages
- tone/type/title/body display
- keyboard response selection
- response submission via `RespondToCommunication`

## Save/load and compatibility

- New diplomacy v3 fields are persisted in `GameState`.
- Save schema bumped to v32.
- v31→v32 migration is passthrough with serde defaults.

## Future expansion path

Planned extensions (not in v3 scope):

- Trade Accord and Research Accord treaty effects
- richer grievance categories and decay windows
- expanded communication templates per faction identity
- deeper diplomacy victory and unity-path hooks
