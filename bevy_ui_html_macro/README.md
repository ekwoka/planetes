# DERELICT

**Station Extraction Prototype - First/Third Person Shooter & Extraction Mechanics**

A focused prototype for testing the core FPS/TPS extraction gameplay loop in abandoned space station environments. Players navigate a derelict station, manage limited resources, loot valuable items, and must extract safely—or lose everything.

---

## Core Concept

Explore a massive abandoned space station in first or third person. Manage your oxygen, inventory weight, and combat threats while looting data, technology, and equipment. The deeper you venture, the better the loot—but extraction points are limited, and death means losing everything from this run.

**Key Pillars:**
- **Tactical Movement:** First and third-person perspectives with smooth camera transitions
- **Resource Management:** Oxygen, ammunition, inventory space
- **Risk vs. Reward:** Push deeper for better loot, or extract safely with what you have
- **Permadeath Extraction:** Death loses everything, safe logout preserves progress

---

## Vertical Slice Features

The vertical slice represents a minimal but complete gameplay loop that captures the core experience. Features are broken down into incremental milestones.

### Milestone 1: Core Movement & Camera
**Goal:** Get basic player movement and perspective switching working

**Features:**
- [ ] Player character spawning and basic transform
- [ ] First-person camera controller
  - Mouse look (pitch/yaw)
  - Smooth camera movement
  - FOV configuration
- [ ] Third-person camera controller
  - Orbit camera with mouse
  - Configurable distance and angles
  - Occlusion handling (camera moves closer when blocked)
- [ ] Perspective toggle (keybind to switch FPS ↔ TPS)
- [ ] Character movement (WASD)
  - Walking speed
  - Running/sprinting (stamina optional for v1)
  - Smooth acceleration/deceleration
- [ ] Jump mechanics
- [ ] Crouch mechanics

**Technical Notes:**
- Use `bevy-tnua` for character controller (already in workspace)
- Use `avian3d` for physics/collision (already in workspace)

---

### Milestone 2: Environment & Collision
**Goal:** Create a small playable station environment

**Features:**
- [ ] Simple station environment mesh (gray-boxed)
  - Corridors
  - Junction room
  - "Loot room" (destination)
  - Extraction room (return point)
- [ ] Collision meshes for all station geometry
- [ ] Basic lighting (point lights, ambient)
- [ ] Spawn point near extraction area
- [ ] Visual markers for key locations
  - Extraction point (green)
  - Loot areas (yellow)
  - Locked/dangerous areas (red)

**Technical Notes:**
- Keep environment simple—focus on gameplay over visuals
- Use modular corridor pieces for easy iteration
- Consider procedural generation hooks for future expansion

---

### Milestone 3: Inventory & Loot System
**Goal:** Implement the core extraction loop mechanics

**Features:**
- [ ] **Inventory System:**
  - Slot-based or weight-based inventory
  - Inventory UI (simple list view)
  - Item pickup interaction (raycast from camera)
  - Item dropping
  - Inventory weight affects movement speed (optional)
- [ ] **Loot Items:**
  - "Data Fragment" (high value, small size)
  - "Tech Component" (medium value, medium size)
  - "Equipment" (low value, large size, usable items)
- [ ] **Item Spawning:**
  - Procedurally placed loot in designated zones
  - Rarity tiers (common, uncommon, rare)
  - Visual distinction (glow colors, models)
- [ ] **Interaction System:**
  - Raycast-based interaction
  - Prompt UI ("Press E to pick up")
  - Range limitation

**Technical Notes:**
- Keep item data simple: name, value, weight, type
- Use Bevy's ECS for flexible item component composition
- Consider future networking: make item IDs deterministic

---

### Milestone 4: Extraction & Session Persistence
**Goal:** Complete the core loop with extraction and safe logout

**Features:**
- [ ] **Extraction Zone:**
  - Green-lit area near spawn
  - Trigger volume detection
  - "Extract" prompt when in zone
  - Extraction animation/timer (3-5 seconds)
  - Cancel extraction if player moves away
- [ ] **Session Persistence:**
  - Save inventory on extraction
  - Save current run state on logout (position, inventory, etc.)
  - Load saved run on game start
  - Separate "extracted" inventory from "current run" inventory
- [ ] **Run Management:**
  - "Start New Run" button (enters station with empty inventory)
  - "Continue Run" button (loads saved position/inventory)
  - Extracted loot visible in "Stash" UI

**Technical Notes:**
- Use simple JSON serialization for save data
- Store extracted loot separately from active run
- Track run statistics (time elapsed, items found, etc.)

---

### Milestone 5: Death & Permadeath
**Goal:** Implement stakes and tension

**Features:**
- [ ] **Health System:**
  - Player health value
  - Health UI (simple bar)
  - Damage sources (hazards, falls, etc.)
- [ ] **Death State:**
  - Detect health <= 0
  - Death screen UI
  - "Run Failed" message
  - Display what was lost (item list)
  - Option to start new run
- [ ] **Permadeath Logic:**
  - Clear current run inventory on death
  - Clear saved run state on death
  - Extracted items remain safe in stash
- [ ] **Environmental Hazards:**
  - Exposed sections (slow damage over time)
  - Pit falls (instant death or heavy damage)
  - Electrical hazards (avoidable damage zones)

**Technical Notes:**
- Keep damage sources environmental for v1 (no AI enemies yet)
- Death should feel fair—clear visual warnings
- Consider death recap/statistics

---

### Milestone 6: Oxygen & Resource Management
**Goal:** Add time pressure and resource management

**Features:**
- [ ] **Oxygen System:**
  - Oxygen value (100% → 0%)
  - Oxygen depletes over time in most areas
  - Safe zones restore oxygen (extraction room, airlocks)
  - Oxygen UI (bar or percentage)
  - Low oxygen warning (audio + visual)
  - Death when oxygen reaches 0
- [ ] **Oxygen Pickups:**
  - Consumable oxygen tanks
  - Found as loot or in fixed locations
  - Instant restore or over-time refill
- [ ] **Zone Types:**
  - Safe zones (green): oxygen restores
  - Stable zones (yellow): oxygen depletes slowly
  - Exposed zones (red): oxygen depletes quickly

**Technical Notes:**
- Oxygen depletion rate should be tuned to create tension without frustration
- Clearly mark zone types with lighting/visual effects
- Consider audio cues (breathing, air hiss)

---

### Milestone 7: Combat Basics (Optional for Vertical Slice)
**Goal:** Add active threats beyond environmental hazards

**Features:**
- [ ] **Simple Weapon:**
  - Pistol or energy weapon
  - Ammo system (finite ammo, found as loot)
  - Projectile system (raycast or physics-based)
  - Weapon UI (ammo counter)
  - Recoil and accuracy mechanics
- [ ] **Basic Enemy:**
  - Patrolling security drone or turret
  - Simple AI (stationary or patrol route)
  - Shoots at player when in range
  - Takes damage and can be destroyed
  - Does not drop loot (keeps scope tight)
- [ ] **Combat Feedback:**
  - Hit markers
  - Damage numbers (optional)
  - Enemy death effects

**Technical Notes:**
- Consider using `avian_bullet_trajectory` (already in workspace)
- Keep AI extremely simple—focus on threat, not complexity
- Combat is optional for vertical slice but adds tension

---

## Vertical Slice Definition

**A complete vertical slice includes Milestones 1-6**, providing a full loop:

1. Player spawns in extraction room
2. Navigates station corridors
3. Finds and collects loot items
4. Manages oxygen in different zones
5. Decides to push deeper or extract
6. Returns to extraction zone and safely extracts
7. **OR** dies and loses everything

**Success Criteria:**
- 5-10 minute gameplay loop
- Clear risk/reward tension (oxygen vs. loot)
- Satisfying extraction moment
- Impactful death state
- Replayability (randomized loot spawns)

---

## Technical Architecture

### Key Systems

**Player Controller:**
- Character movement (bevy-tnua)
- Camera management (first/third person)
- Input handling (bevy_enhanced_input)

**Inventory System:**
- Item components (name, value, weight, type)
- Inventory resource (holds items)
- UI rendering (bevy_ui)

**Loot System:**
- Loot spawner (procedural placement)
- Interaction (raycasting)
- Item pickup/drop

**Session Management:**
- Save/load system (JSON serialization)
- Run state tracking
- Extracted loot stash

**Resource Management:**
- Oxygen system (depleting resource)
- Zone types (oxygen modifiers)
- Health system

**Extraction System:**
- Zone triggers (collision detection)
- Extraction timer
- Run completion

---

## Future Expansion (Post Vertical Slice)

- **Enemies & Combat:** Advanced AI, multiple enemy types, weapon variety
- **Station Complexity:** Multi-level station, locked doors, keycards, puzzles
- **More Resources:** Flashlight battery, suit integrity, radiation
- **Equipment:** Wearable items that affect stats (oxygen capacity, movement speed)
- **Procedural Generation:** Full procedural station layouts
- **Narrative Elements:** Audio logs, environmental storytelling, station lore
- **Multiplayer Hooks:** Asynchronous elements (SOS beacons, player notes, shared market)

---

## Development Notes

- **Iteration Speed:** Prioritize fast iteration—avoid over-engineering
- **Playtesting:** Test frequently to validate fun factor and tension
- **Scope Management:** Cut features ruthlessly to hit vertical slice
- **Visual Polish:** Gray-box is fine for vertical slice; polish later
- **Audio:** Even simple footsteps and ambient audio massively improve feel

---

## Getting Started

```bash
# Run DERELICT prototype
cargo run -p derelict

# Run with dev features (hot reloading, debug tools)
cargo run -p derelict --features dev_native
```

---

## Design Philosophy

**"Get lost to find yourself"**

DERELICT captures the core emotional experience of Planetes: venturing into the unknown, managing limited resources, making tough risk/reward decisions, and the tension of a safe extraction. Every system should reinforce this core loop.

**Focus on:**
- ✅ Tension and release (danger → safety)
- ✅ Meaningful decisions (push deeper vs. extract)
- ✅ Stakes (permadeath, lost progress)
- ✅ Fairness (clear information, player agency)

**Avoid:**
- ❌ Hidden information (unclear dangers)
- ❌ Arbitrary difficulty (unfair deaths)
- ❌ Empty spaces (every area should have purpose)
- ❌ Tedious systems (frustration ≠ tension)
