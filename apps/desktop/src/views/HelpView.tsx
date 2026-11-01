// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  ArrowRight,
  BookOpen,
  BrainCircuit,
  CircleHelp,
  Code2,
  FileCheck2,
  FileText,
  FolderOpen,
  HardDrive,
  Heart,
  ListChecks,
  Layers3,
  LockKeyhole,
  Mail,
  Puzzle,
  RefreshCw,
  ShieldCheck,
  Sparkles,
  TriangleAlert,
  Users,
  WifiOff,
} from "lucide-react";
import type { ReactNode } from "react";
import { PageHeader } from "../components/PageHeader";
import type { Section } from "../types";

export function HelpView({
  onNavigate,
}: {
  onNavigate: (section: Section) => void;
}) {
  return (
    <div className="view help-view">
      <PageHeader
        eyebrow="End-user guide"
        title="Help"
        description="Everything you need to turn a personal export or local archive into readable Markdown—without uploading it or changing the original files."
        action={
          <button
            className="primary-button"
            onClick={() => onNavigate("scan")}
            type="button"
          >
            Start in Scan <ArrowRight size={16} />
          </button>
        }
      />

      <section
        aria-labelledby="help-quick-start"
        className="help-quickstart panel"
      >
        <div className="help-section-heading">
          <div className="help-section-icon">
            <ListChecks size={20} />
          </div>
          <div>
            <span className="eyebrow">The basic workflow</span>
            <h2 id="help-quick-start">Four steps from export to timeline</h2>
          </div>
        </div>
        <ol className="help-steps">
          <li>
            <span className="help-step-number">1</span>
            <div>
              <h3>Choose a source</h3>
              <p>
                Open Scan and select a Takeout, exported mailbox, archive, or
                ordinary folder on your Mac.
              </p>
            </div>
          </li>
          <li>
            <span className="help-step-number">2</span>
            <div>
              <h3>Review what was found</h3>
              <p>
                Unfold shows every recognized route. You decide which source to
                convert.
              </p>
            </div>
          </li>
          <li>
            <span className="help-step-number">3</span>
            <div>
              <h3>Choose a destination</h3>
              <p>
                Pick a separate local folder for the Markdown. Your source
                remains in place.
              </p>
            </div>
          </li>
          <li>
            <span className="help-step-number">4</span>
            <div>
              <h3>Create and review</h3>
              <p>
                Keep Unfold open until it reports completion, then begin with
                All events. It remembers completed export archives together,
                shows the provider first, and lets you open the full Markdown
                body without losing its source evidence.
              </p>
            </div>
          </li>
        </ol>
      </section>

      <div className="help-content">
        <HelpSection
          icon={<Users size={20} />}
          title="Why Unfold exists"
          description="Personal history should belong to the person who created it—not remain trapped inside a company's platform."
        >
          <p className="help-community-lead">
            Some companies have created digital fenced gardens around people's
            photos, messages, journeys, purchases, and memories. Even when an
            export is available, it can be difficult to read or reuse. Amara
            Unfold is a service for everyone: it turns those exports into open,
            understandable files that people can keep, search, move, and build
            on themselves.
          </p>
          <div className="help-guidance-statement" role="note">
            <Heart aria-hidden="true" size={21} />
            <div>
              <span className="eyebrow">
                We show you. We guide you. You choose.
              </span>
              <h3>
                Discover your digital traces. Understand what matters. Conserve
                your memories in open files.
              </h3>
              <p>
                A date, a place, a photograph, a message, a journey, or a sound
                can carry the shape of a life. Unfold makes those traces
                visible, explains where they came from, and helps you decide
                what is worth keeping.
              </p>
            </div>
          </div>
          <div className="help-principles help-community-principles">
            <HelpPrinciple
              icon={<ShieldCheck size={17} />}
              title="Discovery without surveillance"
              text="Unfold is not surveillance, scoring, or a hidden dossier. It does not profile you, infer who you are, or send a portrait of your life anywhere."
            />
            <HelpPrinciple
              icon={<Heart size={17} />}
              title="Conservation by choice"
              text="You choose every source, every module, and every output. Seeing a trace never means you must conserve it; what becomes part of your memory archive remains your decision."
            />
            <HelpPrinciple
              icon={<WifiOff size={17} />}
              title="Fully local. Zero outbound traffic."
              text="Unfold makes no outgoing network requests. There are no uploads, cloud APIs, telemetry, analytics, crash reports, update checks, or background network services."
            />
            <HelpPrinciple
              icon={<BookOpen size={17} />}
              title="Open files, not another lock-in"
              text="The result is Markdown-first, with only small open YAML/CSV indexes where validation or navigation requires them. It remains useful without Unfold, Amara Memory, or any company account."
            />
          </div>
        </HelpSection>

        <HelpSection
          icon={<BookOpen size={20} />}
          title="Why use Amara Unfold?"
          description="Because downloading an export gives you data, but rarely gives you an understandable memory of your life."
        >
          <p className="help-use-lead">
            A Takeout, mailbox, photo folder, listening history, or transaction
            export may contain years of meaningful traces, yet arrive as
            scattered folders, technical tables, and unfamiliar formats. Unfold
            does the foundational work: it makes the sources you choose
            readable, places them on a shared timeline, preserves where every
            record came from, and lets patterns become visible without taking
            ownership away from you.
          </p>
          <div className="help-not-extractor" role="note">
            <Puzzle aria-hidden="true" size={20} />
            <div>
              <strong>This is not metadata extractor number 27</strong>
              <p>
                A conventional extractor often reads fields from one format and
                stops at a technical dump. Unfold treats extraction as the
                beginning: it normalizes different sources into a shared open
                event structure, preserves provenance and readable content,
                validates quality, builds a cross-source timeline and indexes,
                and leaves the result useful outside the app. The goal is not
                more metadata. The goal is usable personal memory.
              </p>
            </div>
          </div>
          <div className="help-use-grid">
            <div className="help-use-card">
              <strong>Find what you forgot was there</strong>
              <p>
                Discover dates, places, journeys, interests, conversations, and
                ordinary moments that are easy to miss inside raw exports.
              </p>
            </div>
            <div className="help-use-card">
              <strong>Understand sources together</strong>
              <p>
                A photograph, song, place, message, or purchase may seem small
                alone. Across time and sources, they can reveal the texture of a
                period in your life.
              </p>
            </div>
            <div className="help-use-card">
              <strong>Create a durable foundation</strong>
              <p>
                Open Markdown remains readable, searchable, editable, and
                portable. It is a foundation you can use today and keep for the
                future.
              </p>
            </div>
          </div>
        </HelpSection>

        <HelpSection
          icon={<BrainCircuit size={20} />}
          title="What can you do with the information?"
          description="The open files are useful by themselves, with a local tool of your choice, or as the foundation for a richer memory experience."
        >
          <div className="help-use-grid help-next-grid">
            <div className="help-use-card">
              <span className="help-use-number">1</span>
              <strong>Read, search, and curate</strong>
              <p>
                Browse the timeline, search the Markdown, correct context, and
                keep only what you want in your memory archive.
              </p>
            </div>
            <div className="help-use-card">
              <span className="help-use-number">2</span>
              <strong>Explore it with a local LLM</strong>
              <p>
                Point a locally running LLM or document tool you trust at the
                files you select. Ask about periods, themes, journeys, or
                changes—and verify its answers against the source links.
              </p>
            </div>
            <div className="help-use-card">
              <span className="help-use-number">3</span>
              <strong>Continue with Amara Memory</strong>
              <p>
                Unfold is the open-source first step and prepares the foundation
                for Amara Memory: your files remain yours while the archive can
                gain visual structure and grounded ways to explore it.
              </p>
            </div>
          </div>

          <div className="help-local-llm" role="note">
            <BrainCircuit aria-hidden="true" size={20} />
            <div>
              <strong>Amara's suggestion: try it with a truly local LLM</strong>
              <p>
                You may be surprised by the richness in the information once
                scattered traces are readable together. The LLM is your separate
                choice—not part of Unfold. Confirm that it stores and indexes
                locally, select only the files you want it to see, and treat its
                interpretation as a suggestion until the cited source supports
                it.
              </p>
            </div>
          </div>

          <div className="help-amara-memory">
            <div className="help-amara-heading">
              <Layers3 aria-hidden="true" size={21} />
              <div>
                <span className="eyebrow">Beyond chat with files</span>
                <h3>Why Amara Memory is more than Markdown plus an LLM</h3>
              </div>
            </div>
            <p className="help-amara-intro">
              Chatting with Markdown can retrieve and summarize pieces of text.
              Amara Memory's role is to turn the same open evidence into an
              explorable picture of a life—without replacing the files as the
              source of truth.
            </p>
            <div className="help-principles help-amara-grid">
              <HelpPrinciple
                icon={<BookOpen size={17} />}
                title="Time becomes a Book"
                text="Days, months, years, and significant periods become a drillable memory structure instead of an unorganized folder of documents."
              />
              <HelpPrinciple
                icon={<Layers3 size={17} />}
                title="Life becomes visible in layers"
                text="People, places, photos, journeys, music, mail, diary, and other chosen domains can be viewed separately or together across a time window."
              />
              <HelpPrinciple
                icon={<Puzzle size={17} />}
                title="Connections stay evidence-based"
                text="Shared dates, people, places, and events can reveal relationships across sources, while facts, inferences, and original evidence remain distinguishable."
              />
              <HelpPrinciple
                icon={<BrainCircuit size={17} />}
                title="Answers can show what they mean"
                text="Grounded questions can lead back to supporting records and drive the visual view, so an answer becomes something you can inspect rather than merely trust."
              />
            </div>
          </div>

          <div className="help-callout help-current-boundary" role="note">
            <TriangleAlert aria-hidden="true" size={18} />
            <div>
              <strong>Honest current boundary</strong>
              <p>
                Unfold is intended to prepare the way into Amara Memory, but
                this build does not contain an LLM, a live import bridge, cloud
                sync, or an Amara account. Continuing remains voluntary, and the
                generated open files remain valuable without it.
              </p>
            </div>
          </div>
        </HelpSection>

        <HelpSection
          icon={<BookOpen size={20} />}
          title="What each module does"
          description="Use the left navigation to move between distinct parts of the local workspace."
        >
          <div className="help-module-grid">
            <HelpModule
              name="Home"
              text="See a private summary of indexed files, recent sources, and photo-event drafts."
            />
            <HelpModule
              name="Scan"
              text="Choose source data, confirm detected formats, select an output folder, and create Markdown."
            />
            <HelpModule
              name="Memories"
              text="Inspect conservative photo-event groupings and the file metadata supporting each draft."
            />
            <HelpModule
              name="Settings"
              text="Choose the appearance and verify privacy, network, telemetry, database, and version status."
            />
          </div>
        </HelpSection>

        <HelpSection
          icon={<FolderOpen size={20} />}
          title="Sources you can use"
          description="Unfold works from files and exports you control. It does not sign into cloud accounts."
        >
          <div className="help-source-grid">
            <div className="help-source-group">
              <h3>Ready-to-select data</h3>
              <ul>
                <li>Photo and video folders</li>
                <li>Google Takeout for Photos, Gmail, YouTube, and Maps</li>
                <li>Airbnb JSON exports</li>
                <li>Banking CSV, XLSX, XLS, and ODS exports</li>
                <li>Diary DOCX and place CSV/JSON/KMZ</li>
              </ul>
            </div>
            <div className="help-source-group">
              <h3>Mac and local-app exports</h3>
              <ul>
                <li>Apple Photos exported originals or media folders</li>
                <li>
                  Apple Mail exported <code>.mbox</code> packages
                </li>
                <li>
                  Apple Calendar <code>.ics</code> exports
                </li>
                <li>
                  Apple Contacts <code>.vcf</code> exports
                </li>
                <li>Apple Music metadata and local files</li>
              </ul>
            </div>
          </div>
          <div className="help-callout" role="note">
            <TriangleAlert aria-hidden="true" size={18} />
            <div>
              <strong>Not available in this build</strong>
              <p>
                Outlook PST, Immich, Apple Messages, Apple Notes, Apple Journal,
                and NotebookLM Takeouts are not working import routes. Unfold
                does not read their private stores.
              </p>
            </div>
          </div>
        </HelpSection>

        <HelpSection
          icon={<FileText size={20} />}
          title="What Unfold creates"
          description="The result is ordinary, portable Markdown with enough provenance to audit where each event came from."
        >
          <div className="help-split">
            <div>
              <h3>For each successfully converted source</h3>
              <ul>
                <li>
                  <code>README.md</code> explains the package.
                </li>
                <li>
                  <code>manifest.yaml</code> records counts and source details.
                </li>
                <li>
                  <code>events/YYYY/MM/*.md</code> contains readable events.
                </li>
                <li>Quality reports identify anything that needs review.</li>
              </ul>
            </div>
            <div>
              <h3>Across all successfully converted sources</h3>
              <ul>
                <li>
                  <code>Amara-Unfold-Timeline/index.md</code> is the starting
                  page.
                </li>
                <li>
                  Day, month, year, and undated pages plus entity indexes for
                  people, places, merchants, artists, and channels aid
                  navigation.
                </li>
                <li>
                  <code>catalog.csv</code> provides a compact inventory.
                </li>
                <li>
                  Connections are evidence-based and never presented as
                  certainty.
                </li>
              </ul>
            </div>
          </div>
          <div className="help-path">
            <span>Typical package location</span>
            <code>
              &lt;destination&gt;/&lt;service&gt;/&lt;export-date&gt;/
            </code>
          </div>
        </HelpSection>

        <HelpSection
          icon={<ShieldCheck size={20} />}
          title="Privacy and safe use"
          description="The app is designed to make local behavior visible and predictable."
        >
          <div className="help-principles">
            <HelpPrinciple
              icon={<LockKeyhole size={17} />}
              title="Zero traffic leaves the app"
              text="All supported extraction, indexing, timeline generation, and memory grouping run on this device without an internet connection."
            />
            <HelpPrinciple
              icon={<HardDrive size={17} />}
              title="Originals stay untouched"
              text="Unfold reads selected data and writes to a destination you choose. It does not move or delete source files."
            />
            <HelpPrinciple
              icon={<FileCheck2 size={17} />}
              title="Index removal is limited"
              text="Removing a source clears its local index records and drafts—not the original folder or generated Markdown."
            />
            <HelpPrinciple
              icon={<Sparkles size={17} />}
              title="Evidence before inference"
              text="Memory drafts use dates, folders, GPS, and device metadata. They do not identify people or invent stories."
            />
          </div>
          <p className="help-privacy-note">
            The local SQLite index and generated Markdown may contain personal
            paths or content. Treat both as private data and store them on a
            protected disk.
          </p>
        </HelpSection>

        <HelpSection
          icon={<Puzzle size={20} />}
          title="Future modules to build"
          description="These are ideas for community work, not capabilities in this build."
        >
          <div className="help-contribute-grid">
            <div className="help-contribute-card">
              <h3>Podcasts</h3>
              <p>
                A future local module could help people preserve selected
                podcast subscriptions and listening history without contacting a
                service.
              </p>
            </div>
            <div className="help-contribute-card">
              <h3>NotebookLM</h3>
              <p>
                An offline adapter may be considered if a stable, open export
                format becomes available. It is not supported today.
              </p>
            </div>
          </div>
        </HelpSection>

        <HelpSection
          icon={<Code2 size={20} />}
          title="Build with us"
          description="Unfold is open source so the community can inspect it and improve the reliability, privacy, and clarity of the frozen feature set."
        >
          <div className="help-contribute-grid">
            <div className="help-contribute-card">
              <h3>GNU AGPL v3.0 or later</h3>
              <p>
                Amara Unfold is released under the GNU Affero General Public
                License v3.0 or later. You are welcome to inspect, use, modify,
                and share the software under the terms of that license.
              </p>
            </div>
            <div className="help-contribute-card">
              <h3>Reliability work is welcome</h3>
              <p>
                Contributions can strengthen existing adapters, validators,
                tests, accessibility, and documentation while preserving the
                local-first privacy boundary and frozen release scope.
              </p>
            </div>
          </div>
          <div className="help-contact" role="note">
            <Mail aria-hidden="true" size={18} />
            <div>
              <strong>Want to contribute a focused improvement?</strong>
              <p>
                Write to <a href="mailto:info@amara.gmbh">info@amara.gmbh</a>.
                Unfold itself does not send the message; the link only opens
                your chosen mail application after you click it.
              </p>
            </div>
          </div>
        </HelpSection>

        <HelpSection
          icon={<RefreshCw size={20} />}
          title="Troubleshooting"
          description="Most problems come from selecting the wrong export level, missing read permission, or an incomplete export."
        >
          <div className="help-troubleshooting">
            <TroubleshootingItem
              problem="The folder is not recognized"
              answer="Choose the actual export folder or its immediate parent. For Mail select a .mbox package; for Calendar use .ics; for Contacts use .vcf."
            />
            <TroubleshootingItem
              problem="A recent export is not shown"
              answer="Return to Scan and choose Refresh app exports, or select the exported folder manually."
            />
            <TroubleshootingItem
              problem="Records were skipped or need review"
              answer="Open the package quality report and manifest. Skips remain named and counted; they are not silently discarded."
            />
            <TroubleshootingItem
              problem="An extraction was interrupted"
              answer="Run the same route again. Deterministic filenames let completed events remain valid while existing events are skipped safely."
            />
            <TroubleshootingItem
              problem="macOS blocks the app"
              answer="The current candidate is ad-hoc signed, not Developer ID signed, and not notarized. Build from source, or use Finder's Control-click Open flow only for a separately labelled candidate you trust."
            />
            <TroubleshootingItem
              problem="You need to report a problem"
              answer="Share the exact error and app version, but remove personal paths and never publish raw exports, private Markdown, or the local database."
            />
          </div>
        </HelpSection>

        <HelpSection
          icon={<CircleHelp size={20} />}
          title="Frequently asked questions"
          description="Short answers to the questions that matter before using personal archives."
        >
          <div className="help-faq">
            <details>
              <summary>Does Unfold copy my photos or source archives?</summary>
              <p>
                No. It reads the source in place and writes Markdown plus
                reports to the destination you select. A media Markdown export
                references originals; it does not duplicate them.
              </p>
            </details>
            <details>
              <summary>Can I open the output without Amara Memory?</summary>
              <p>
                Yes. The output is Markdown-first, with only small open YAML/CSV
                indexes where validation or navigation requires them. It remains
                useful without Unfold or any future Amara product.
              </p>
            </details>
            <details>
              <summary>Does Unfold need internet access?</summary>
              <p>
                No. Supported extraction, indexing, timeline generation, and
                memory drafts run locally with zero outbound traffic. Unfold
                does not download exports for you.
              </p>
            </details>
            <details>
              <summary>What are the important current limitations?</summary>
              <p>
                The current macOS candidate is ad-hoc signed, not Developer ID
                signed, and not notarized. There is no cloud sync, live Amara
                Memory bridge, face recognition, or AI-written story generation.
                Video duration is not yet extracted, HEIC metadata depends on
                decoder support, and the SQLite index is not application-level
                encrypted.
              </p>
            </details>
          </div>
        </HelpSection>
      </div>
    </div>
  );
}

function HelpSection({
  icon,
  title,
  description,
  children,
}: {
  icon: ReactNode;
  title: string;
  description: string;
  children: ReactNode;
}) {
  const id = `help-${title.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`;
  return (
    <section aria-labelledby={id} className="help-topic panel">
      <div className="help-section-heading">
        <div className="help-section-icon">{icon}</div>
        <div>
          <h2 id={id}>{title}</h2>
          <p>{description}</p>
        </div>
      </div>
      <div className="help-topic-body">{children}</div>
    </section>
  );
}

function HelpModule({ name, text }: { name: string; text: string }) {
  return (
    <div className="help-module">
      <strong>{name}</strong>
      <p>{text}</p>
    </div>
  );
}

function HelpPrinciple({
  icon,
  title,
  text,
}: {
  icon: ReactNode;
  title: string;
  text: string;
}) {
  return (
    <div className="help-principle">
      <span>{icon}</span>
      <div>
        <strong>{title}</strong>
        <p>{text}</p>
      </div>
    </div>
  );
}

function TroubleshootingItem({
  problem,
  answer,
}: {
  problem: string;
  answer: string;
}) {
  return (
    <div className="help-troubleshooting-item">
      <strong>{problem}</strong>
      <p>{answer}</p>
    </div>
  );
}
