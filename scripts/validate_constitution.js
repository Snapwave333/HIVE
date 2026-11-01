import fs from "fs";
import path from "path";

console.log("=================================================");
console.log("   H.I.V.E. Application Constitution Validator   ");
console.log("=================================================");

const rootDir = process.cwd();
const constitutionPath = path.join(rootDir, "app", "constitution.json");

if (!fs.existsSync(constitutionPath)) {
  console.error("FAIL: constitution.json not found at:", constitutionPath);
  process.exit(1);
}

const constitution = JSON.parse(fs.readFileSync(constitutionPath, "utf-8"));
console.log(`Loaded Constitution v${constitution.version}`);

let violations = 0;

// Rule 1: User sovereignty
if (!constitution.invariants.user_sovereignty?.immutable) {
  console.error("FAIL: user_sovereignty invariant missing or mutated!");
  violations++;
} else {
  console.log("✓ Invariant Passed: User Sovereignty locked.");
}

// Rule 2: Agency Chat Architecture (1 HQ + 12 DMs)
if (!constitution.invariants.agency_chat_architecture?.immutable) {
  console.error("FAIL: agency_chat_architecture invariant missing!");
  violations++;
} else {
  console.log("✓ Invariant Passed: Agency Chat Architecture (#agency-hq + 12 DMs).");
}

// Rule 3: 12 Permanent Agent Permanence
if (!constitution.invariants.agent_permanence?.immutable) {
  console.error("FAIL: agent_permanence invariant missing!");
  violations++;
} else {
  console.log("✓ Invariant Passed: 12 Permanent Agent Permanence.");
}

// Rule 4: WhatsApp-style messaging interaction model
if (!constitution.invariants.messaging_interaction_model?.immutable) {
  console.error("FAIL: messaging_interaction_model invariant missing!");
  violations++;
} else {
  console.log("✓ Invariant Passed: WhatsApp-style messaging UX.");
}

// Rule 5: Audio and notification sovereignty (strictly muted by default)
if (!constitution.invariants.audio_and_notification_sovereignty?.immutable) {
  console.error("FAIL: audio_and_notification_sovereignty invariant missing!");
  violations++;
} else {
  console.log("✓ Invariant Passed: Audio muted by default.");
}

// Rule 6: Accessibility baseline (WCAG AA, ADHD 5m task chunking)
if (!constitution.invariants.accessibility_baseline?.immutable) {
  console.error("FAIL: accessibility_baseline invariant missing!");
  violations++;
} else {
  console.log("✓ Invariant Passed: Accessibility baseline & ADHD 5m chunking.");
}

// Rule 7: Zero dashboard bloat (prohibits 3D virtual environment)
if (!constitution.invariants.zero_dashboard_bloat?.immutable) {
  console.error("FAIL: zero_dashboard_bloat invariant missing!");
  violations++;
} else {
  console.log("✓ Invariant Passed: Zero Dashboard Bloat / No 3D worlds.");
}

// AST/File scan for forbidden regressions
const forbiddenTerms = ["Superpowers", "Metaverse", "VirtualWorld3D"];
const sourceFiles = [
  path.join(rootDir, "src", "App.tsx"),
  path.join(rootDir, "src", "components", "NavRail.tsx"),
  path.join(rootDir, "src", "components", "ConversationList.tsx"),
];

for (const file of sourceFiles) {
  if (fs.existsSync(file)) {
    const content = fs.readFileSync(file, "utf-8");
    for (const term of forbiddenTerms) {
      if (content.includes(term)) {
        console.error(`FAIL: Forbidden regression term '${term}' found in ${path.basename(file)}!`);
        violations++;
      }
    }
  }
}

if (violations === 0) {
  console.log("\n>>> ALL CONSTITUTION INVARIANTS VERIFIED CLEAN. ZERO VIOLATIONS. <<<");
  process.exit(0);
} else {
  console.error(`\n>>> FAILED: ${violations} CONSTITUTION VIOLATION(S) DETECTED! <<<`);
  process.exit(1);
}
