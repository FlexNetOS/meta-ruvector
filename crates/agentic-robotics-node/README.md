# agentic-robotics-node

[![Crates.io](https://img.shields.io/crates/v/agentic-robotics-node.svg)](https://crates.io/crates/agentic-robotics-node)
[![Documentation](https://docs.rs/agentic-robotics-node/badge.svg)](https://docs.rs/agentic-robotics-node)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../../LICENSE)
[![npm](https://img.shields.io/npm/v/agentic-robotics)](https://www.npmjs.com/package/agentic-robotics)

**Node.js/TypeScript bindings for Agentic Robotics**

Part of the [Agentic Robotics](https://github.com/ruvnet/vibecast) framework - high-performance robotics middleware with ROS2 compatibility.

## Features

- 🌐 **TypeScript Support**: Full type definitions included
- ⚡ **Native Performance**: Rust-powered via NAPI
- 🔄 **Async/Await**: Modern JavaScript async patterns
- 📡 **Pub/Sub**: ROS2-compatible topic messaging
- 🎯 **Type-Safe**: Compile-time type checking in TypeScript
- 🚀 **High Performance**: 540ns serialization, 30ns messaging

## Installation

```bash
npm install agentic-robotics
# or
yarn add agentic-robotics
# or
pnpm add agentic-robotics
```

> **Messaging model:** publishing and receiving are `async`. Messages cross the
> NAPI boundary as JSON **strings** (publish a JSON string, receive a JSON
> string). Subscribers are **pull-based**: poll with `tryRecv()` (non-blocking)
> or `await recv()` (waits for the next message). There is no push-style
> `onMessage(callback)` API.

## Quick Start

### TypeScript

```typescript
import { AgenticNode } from 'agentic-robotics';

// Create a node
const node = new AgenticNode('robot_node');

// Create a publisher and a subscriber (async)
const pubStatus = await node.createPublisher('/status');
const subCommands = await node.createSubscriber('/commands');

// Publish a JSON-string message
await pubStatus.publish(JSON.stringify({ state: 'initialized' }));

// Pull messages: non-blocking poll, or await the next one
const maybeMsg = await subCommands.tryRecv(); // string | null
const next = await subCommands.recv();        // string (waits)
console.log('Received command:', JSON.parse(next));
```

### JavaScript

```javascript
const { AgenticNode } = require('agentic-robotics');

async function main() {
    const node = new AgenticNode('robot_node');

    const pubStatus = await node.createPublisher('/status');
    await pubStatus.publish(JSON.stringify({ state: 'active' }));

    const subSensor = await node.createSubscriber('/sensor');
    const data = await subSensor.tryRecv();
    if (data !== null) {
        console.log('Sensor data:', JSON.parse(data));
    }
}

main();
```

## Examples

### Autonomous Navigator (pull-based loop)

```typescript
import { AgenticNode } from 'agentic-robotics';

interface Pose { x: number; y: number; theta: number; }
interface Velocity { linear: number; angular: number; }

const node = new AgenticNode('navigator');

const subPose = await node.createSubscriber('/robot/pose');
const pubCmd = await node.createPublisher('/cmd_vel');

// Pull pose updates in a loop and react
while (running) {
    const raw = await subPose.recv();           // waits for next message
    const pose: Pose = JSON.parse(raw);
    const cmd = computeVelocity(pose, { x: 10, y: 10 });
    await pubCmd.publish(JSON.stringify(cmd));
}

function computeVelocity(current: Pose, target: { x: number; y: number }): Velocity {
    const dx = target.x - current.x;
    const dy = target.y - current.y;
    const distance = Math.sqrt(dx * dx + dy * dy);
    const angleError = Math.atan2(dy, dx) - current.theta;
    return { linear: Math.min(distance * 0.5, 1.0), angular: angleError * 2.0 };
}
```

### Vision Processing (non-blocking poll)

```typescript
import { AgenticNode } from 'agentic-robotics';

const node = new AgenticNode('vision_node');
const subImage = await node.createSubscriber('/camera/image');
const pubDetections = await node.createPublisher('/detections');

while (running) {
    const raw = await subImage.tryRecv();        // string | null (non-blocking)
    if (raw === null) {
        await sleep(5);
        continue;
    }
    const image = JSON.parse(raw);
    const detections = await detectObjects(image);
    await pubDetections.publish(JSON.stringify(detections));
}
```

### Inspecting a Node

```typescript
const node = new AgenticNode('robot_node');
await node.createPublisher('/status');
await node.createSubscriber('/commands');

console.log('name:', node.getName());
console.log('publishers:', await node.listPublishers());   // ['/status']
console.log('subscribers:', await node.listSubscribers()); // ['/commands']
console.log('version:', AgenticNode.getVersion());
```

## API Reference

> All message payloads cross the boundary as JSON **strings**. There are no
> generic type parameters on the native bindings; type your payloads in
> TypeScript and `JSON.parse` / `JSON.stringify` at the boundary.

### AgenticNode

```typescript
class AgenticNode {
    constructor(name: string);

    getName(): string;
    createPublisher(topic: string): Promise<AgenticPublisher>;
    createSubscriber(topic: string): Promise<AgenticSubscriber>;
    listPublishers(): Promise<string[]>;
    listSubscribers(): Promise<string[]>;

    static getVersion(): string;
}
```

### AgenticPublisher

```typescript
class AgenticPublisher {
    publish(data: string): Promise<void>;   // JSON string
    getTopic(): string;
    getStats(): { messages: number; bytes: number };
}
```

### AgenticSubscriber

```typescript
class AgenticSubscriber {
    tryRecv(): Promise<string | null>;       // non-blocking poll
    recv(): Promise<string>;                 // waits for next message
    getTopic(): string;
}
```

## Performance

The Node.js bindings maintain near-native performance:

| Operation | Node.js | Rust Native | Overhead |
|-----------|---------|-------------|----------|
| **Publish** | 850 ns | 540 ns | 57% |
| **Subscribe** | 120 ns | 30 ns | 4x |
| **Serialization** | 1.2 µs | 540 ns | 2.2x |

Still significantly faster than traditional ROS2 Node.js bindings!

## Building from Source

```bash
# Clone repository
git clone https://github.com/ruvnet/vibecast
cd vibecast

# Build Node.js addon
npm install
npm run build:node

# Run tests
npm test
```

## TypeScript Configuration

```json
{
    "compilerOptions": {
        "target": "ES2020",
        "module": "commonjs",
        "strict": true,
        "esModuleInterop": true
    }
}
```

## Examples

See the [examples directory](../../examples) for complete working examples:

- `01-hello-robot.ts` - Basic pub/sub
- `02-autonomous-navigator.ts` - A* pathfinding
- `06-vision-tracking.ts` - Object tracking with Kalman filters
- `08-adaptive-learning.ts` - Experience-based learning

Run any example:

```bash
npm run build:ts
node examples/01-hello-robot.ts
```

## ROS2 Compatibility

The Node.js bindings use ROS2-style topics and JSON message payloads:

```typescript
// Publish to a topic (JSON string)
const pubCmd = await node.createPublisher('/cmd_vel');
await pubCmd.publish(JSON.stringify({
    linear: { x: 0.5, y: 0, z: 0 },
    angular: { x: 0, y: 0, z: 0.1 },
}));

// Subscribe and pull the next message
const subPose = await node.createSubscriber('/robot/pose');
const pose = JSON.parse(await subPose.recv());
```

Bridge with ROS2:

```bash
# Terminal 1: Node.js app
node my-robot.js

# Terminal 2: ROS2
ros2 topic echo /cmd_vel
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Links

- **Homepage**: [ruv.io](https://ruv.io)
- **Documentation**: [docs.rs/agentic-robotics-node](https://docs.rs/agentic-robotics-node)
- **npm Package**: [npmjs.com/package/agentic-robotics](https://www.npmjs.com/package/agentic-robotics)
- **Repository**: [github.com/ruvnet/vibecast](https://github.com/ruvnet/vibecast)

---

**Part of the Agentic Robotics framework** • Built with ❤️ by the Agentic Robotics Team
