# Scenario: Senior Engineer's Month

## Overview

Simulate one month of work for **Alex Chen**, a Senior Software Engineer at a mid-size SaaS company called **Meridian**. Alex leads a team of 3 engineers, reports to a Director of Engineering, and splits time across a major project, several smaller projects, and leadership/team responsibilities.

This scenario tests the **stacks** concept by creating tasks across multiple stacks representing real workstreams. It also tests task lifecycle (create, update, complete, delete), dependencies, priorities, tags, and search.

## How to Run

Each agent should use the `stackstodo` CLI (at `./target/release/stackstodo`) with `STACKSTODO_DIR` set to a temp directory. Agents should:

1. Create tasks as they "discover" work throughout the simulated month
2. Use stacks to organize by project/workstream
3. Update task status as work progresses (todo -> in_progress -> done)
4. Use priorities, tags, and due dates realistically
5. Complete or cancel tasks as the month unfolds
6. Create dependencies between tasks where natural (e.g. "deploy" blocked by "write tests")

Agents should NOT create all tasks upfront. Simulate organic discovery: start week 1, create tasks, work some, then week 2 brings new tasks, interrupts, etc.

## Stacks

| Stack | Description |
|-------|-------------|
| `atlas` | The big full-stack project |
| `internal-tools` | Smaller internal projects, developer tooling |
| `bugs` | Cross-project bug fixes and incidents |
| `leadership` | Team lead duties: reviews, mentoring, meetings |
| `personal` | Career growth, learning, 1:1 prep |

## Agent Roles

### Agent 1: "leadership-agent" — Team & Leadership Work

Simulates Alex's responsibilities as a team lead and employee within the org.

**Work includes:**
- Sprint planning, retros, standups
- Code reviews for team members (3 engineers: Jamie, Priya, Sam)
- 1:1s with direct reports and skip-level with Director
- Performance review season (mid-cycle check-ins)
- Mentoring a junior engineer (Sam) on system design
- Responding to cross-team requests and Slack threads
- Writing a quarterly team update / roadmap doc
- Interviewing candidates (2 phone screens, 1 onsite this month)
- Personal: prepping a conference talk proposal, reading a technical book

**Stacks used:** `leadership`, `personal`

### Agent 2: "atlas-agent" — Major Project Work

Simulates Alex's hands-on engineering work on **Project Atlas**, the company's new real-time analytics dashboard.

**Project Atlas:**
- Full-stack: React + TypeScript frontend, Rust backend (actix-web), PostgreSQL + TimescaleDB
- Current state: Backend API is ~70% done. Frontend has basic scaffolding. No CI/CD yet. Team is 2 weeks into a 3-month timeline.
- This month's goals: finish the query engine, build the first 3 dashboard widgets, set up CI/CD, write integration tests

**Work includes:**
- Designing and implementing the query aggregation engine
- Building React dashboard components (time-series chart, KPI cards, filter bar)
- Setting up GitHub Actions CI/CD pipeline
- Writing integration tests for the API layer
- Reviewing and merging PRs from Priya (frontend) and Jamie (data pipeline)
- Fixing performance issues found during load testing
- Writing ADR (Architecture Decision Record) for caching strategy
- Handling a mid-month scope change request from product

**Stacks used:** `atlas`

### Agent 3: "misc-agent" — Smaller Projects & Cross-Cutting Work

Simulates the scattered, interrupt-driven work across smaller projects and internal tooling.

**Projects:**

- **Meridian CLI** (`internal-tools`): Internal developer CLI for provisioning test environments. Current state: works but slow, needs a caching layer and better error messages.
- **Auth Service** (`internal-tools`): Shared authentication microservice. Current state: stable but needs OAuth2 PKCE flow added for a new mobile app.
- **Bug fixes** (`bugs`): Production incidents and bug reports that come in throughout the month.
- **DevEx improvements** (`internal-tools`): Docker compose optimization, shared ESLint config, monorepo migration investigation.

**Work includes:**
- Adding caching to the internal CLI tool
- Implementing OAuth2 PKCE flow in auth service
- Investigating and fixing a production memory leak (week 2 interrupt)
- Responding to 2-3 bug reports from support team
- Improving Docker compose startup time
- Publishing shared ESLint config as internal npm package
- Writing RFC for monorepo migration
- Upgrading a dependency with a CVE across 4 services

**Stacks used:** `internal-tools`, `bugs`

## Timeline Guidance

Agents should roughly follow this timeline but improvise details:

- **Week 1:** Planning, initial task creation, starting work. Leadership agent does sprint planning.
- **Week 2:** Deep work. A production incident interrupts. Mid-sprint check-in.
- **Week 3:** Scope change on Atlas. Performance review prep. Conference talk deadline.
- **Week 4:** Sprint wrap-up, demos, retro. Complete and clean up tasks. Plan next month.

## Expected Outcomes

After the simulation, we should see:
- 40-80 tasks created across 5+ stacks
- Mix of statuses: ~60% done, ~20% in_progress, ~15% todo, ~5% cancelled
- Realistic use of priorities (most medium, some high, few critical)
- Tags used for cross-cutting concerns (e.g. `security`, `frontend`, `backend`, `ci`, `docs`)
- Some task dependencies (especially in Atlas project)
- Due dates on deadline-driven tasks
- Search should find tasks across projects by keyword

## Scaling Notes

To run with more agents or bigger scope:
- Add agents for each additional engineer on the team
- Add more projects with their own stacks
- Increase the timeline (quarter instead of month)
- Add a "product manager" agent that creates feature requests and interrupts
- Add an "oncall" agent that creates incident tasks mid-simulation
