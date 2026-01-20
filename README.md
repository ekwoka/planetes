# Planetes (Wanderer)

> Planetes Editor
> If you're here for just the Bevy game editor, head to the [planetes_editor crate](/planetes_editor/) for more information.

**A Massively Single-Player extraction game about braving the unknown depths of procedurally-collapsed space**

> *Like sailors venturing beyond charted waters, you dive into unstable wormhole networks where the path home is never guaranteed.*



---

## Overview

Planetes is an ambitious exploration-focused game built in Bevy, combining extraction shooter mechanics with deep space exploration through dynamic wormhole networks. Players venture into the unknown on multi-day persistent runs, risking everything to retrieve valuable data, technology, and artifacts from abandoned stations and deep space anomalies.

**Development Philosophy:** Breaking down complex systems into standalone prototype games, each testing core mechanics in isolation before integration into the final experience.

### Core Concept
- **Permadeath Extraction:** Everything lost on death, but safe logout between sessions
- **Dynamic Wormholes:** Procedural networks that open/close based on conditions—return paths are never guaranteed
- **Massively Single-Player:** Asynchronous multiplayer impact through shared markets, wormhole stabilization, and player managed infrastructure
- **Depth & Risk:** Venture deeper for greater rewards, but lose all sense of how to return

### Inspirations
- **EVE Online:** Economic depth, ship fitting, persistent universe impact
- **Stargate:** Wormhole exploration, unknown worlds, ancient technology
- **Made in Abyss:** Depth-based progression, unknown return, environmental storytelling
- **War Thunder:** Realistic vehicle simulation, complex combat systems

---

## Project Structure

This repository contains both the standalone prototype games and the final integrated experience. Components are **public source**, while the final game will be **source available for game owners only**.

### Platforms
- PC (Windows/Linux)
- macOS

### Current Status
🔵 **Concept Phase** - Individual prototypes in development

---

## The Prototype Games

Each prototype is a focused, playable game in its own right, designed to test and refine specific mechanics before integration.

### 🎯 **DERELICT** - Station Extraction Prototype
*First/Third-Person Shooter & Extraction Mechanics*

**Concept:** Explore a single massive abandoned space station, looting equipment and data while managing inventory, oxygen, and hostiles (environmental hazards, security systems, or other threats).

**Core Systems Tested:**
- First and third-person combat mechanics
- Inventory management under pressure
- Environmental navigation (zero-G sections, locked areas, hazards)
- Extraction points and safe logout mechanics
- Risk/reward decision-making (push deeper vs. extract safely)

**Win Condition:** Successfully extract with loot
**Loss Condition:** Death = lose everything from this run

---

### 🚀 **NAVIGATOR** - Ship Flight & Wormhole Prototype
*Space Flight, Combat, and Wormhole Navigation*

**Concept:** Pilot a single-crew ship through a small procedurally-connected wormhole network. Navigate between nodes, engage in ship combat, estimate wormhole "depth," and manage fuel/supplies to find your way back.

**Core Systems Tested:**
- Ship flight physics and controls
- Ship-to-ship combat
- Wormhole visualization and depth estimation
- Procedural network generation from handcrafted scenarios
- Resource management (fuel, ammo, supplies)
- Interior ship spaces and functionality
- Safe logout within ship

**Win Condition:** Successfully navigate home with discoveries
**Loss Condition:** Stranded without fuel/supplies, destroyed in combat

---

### 🏭 **FOUNDRY** - Industry & Automation Prototype
*Resource Processing, Factory Building, and Market Systems*

**Concept:** Build and optimize a resource processing facility. Mine raw materials, research blueprints from recovered data, manufacture items, and engage with the open market.

**Core Systems Tested:**
- Factory building and automation chains
- Resource gathering (mining mechanics)
- Research and blueprint progression
- Item crafting and manufacturing
- Market simulation (buy/sell, supply/demand)
- Idle progression and optimization

**Win Condition:** Build efficient production chains, profit from market trading
**Loss Condition:** Economic failure (optional failure state for testing)

---

### 🌌 **PLANETES** - The Final Game
*All Systems Integrated*

**The Complete Vision:** Combine extraction shooting, ship flight, and industry into a cohesive loop:

1. **Industry Phase (Safe Zone):** Build equipment, research tech, prepare for expeditions
2. **Expedition Phase (Wormhole Space):**
   - Navigate dynamic wormhole networks in your ship
   - Land at/dock with derelict stations for FPS extraction gameplay
   - Manage ship resources and plot courses through unstable space
   - Find valuable data, technology, and items
3. **Return & Risk:** Dynamic wormholes mean paths collapse—you must adapt or be stranded
4. **Massively Single-Player:** Your actions affect other players asynchronously:
   - Items sold on market become available to others
   - Stabilized wormholes may appear in other players' games
   - SOS beacons create rescue opportunities
   - Research breakthroughs shared across the player base

**Aesthetic Journey:** As players venture deeper, environments shift in tone—from familiar industrial stations to ancient alien structures, capturing the feeling of sailing beyond all known charts.

---

## Development roadmap

### Phase 1: Individual Prototypes (Current)
- [ ] DERELICT - FPS/TPS extraction mechanics
- [ ] NAVIGATOR - Ship flight and wormhole systems
- [ ] FOUNDRY - Industry and market systems

### Phase 2: Vertical Slice
- [ ] Integrate one complete gameplay loop (industry → expedition → extraction → return)
- [ ] Test core risk/reward balance
- [ ] Validate "massively single-player" async systems

### Phase 3: Content & Polish
- [ ] Expand handcrafted scenario pool for procedural generation
- [ ] Environmental storytelling and lore integration
- [ ] Audio/visual polish for "braving the unknown" aesthetic

### Phase 4: Release
- [ ] Final balancing and optimization
- [ ] Platform builds (PC, macOS)
- [ ] Open source release for game owners

---

## Technical Stack

- **Engine:** Bevy (Rust)
- **Languages:** Rust
- **Platforms:** PC (Windows/Linux), macOS

---

## Contributing

This project is in early concept/prototype phase. Contribution guidelines will be established as individual prototypes solidify.

**Component Licensing:** Public source (free to use and learn from)
**Final Game Licensing:** Open source for game owners only

---

## Lore Hook

*The wormholes appeared without warning. Ancient, stable, leading to star systems light-years away in an instant. Humanity rushed through them, colonizing, exploiting, building.*

*Then they began to collapse. Not all at once, but unpredictably. Stations fell silent. Colonies were cut off. The networks became mazes of unstable passages—some opening for hours, others collapsing in minutes.*

*Now only the brave venture into wormhole space. Scavengers, researchers, desperados. They dive deep, knowing every jump takes them further from safety. Knowing the path home may be gone when they turn back.*

*They are the Planetes. The wanderers. And they always venture deeper.*

---

## Contact & Support

*TBD - Project in concept phase*
