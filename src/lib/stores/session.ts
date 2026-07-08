import { writable, get } from 'svelte/store';
import {
    getNextQuestion,
    submitAnswer,
    getQueueCounts,
    type QuestionDto,
    type QueueCounts
} from '$lib/api/quiz';

export type Mode = 'multiple' | 'type';

// How the SRS queue is scoped for a session:
//   mixed  — due reviews + new up to the cap (the daily driver)
//   review — clear due cards only, introduce no new
//   new    — introduce new birds only, skip the review queue
//   cram   — practice anyway: ignore the schedule, fixed `length` questions
export type StudyMode = 'mixed' | 'review' | 'new' | 'cram';

export type SessionConfig = {
    pack: string; // '' = whole library
    length: number; // Anki new-card cap: max brand-new birds introduced this session
    mode: Mode;
    study: StudyMode;
};

export type AnswerRecord = {
    bird_id: number;
    recording_id: number; // kept so results can replay the call
    guess: string;
    correct: boolean;
    correct_name: string;
    skipped: boolean;
};

export type SessionState = {
    id: string;
    config: SessionConfig;
    index: number; // 0-based question pointer (counts every card shown, incl. reviews)
    newServed: number; // brand-new birds introduced so far (against config.length cap)
    answers: AnswerRecord[];
    current: QuestionDto | null; // current question, choices already shuffled
    finished: boolean; // truly over — go to the results screen
    extended: boolean; // user chose to keep training past the scheduled queue (study-ahead)
    remaining: QueueCounts | null; // eligible cards left (null for cram / extended / not counted)
};

export const session = writable<SessionState | null>(null);

// Fisher–Yates; Rust appends the correct answer last, so the UI shuffles.
function shuffle<T>(arr: T[]): T[] {
    const out = [...arr];
    for (let i = out.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [out[i], out[j]] = [out[j], out[i]];
    }
    return out;
}

export function startSession(id: string, config: SessionConfig) {
    session.set({
        id,
        config,
        index: 0,
        newServed: 0,
        answers: [],
        current: null,
        finished: false,
        extended: false,
        remaining: null
    });
}

// The user cleared their scheduled cards and chose to keep going: switch to
// study-ahead (pull upcoming cards regardless of due date), like a rolling cram.
export function continueTraining() {
    session.update((s) => (s ? { ...s, extended: true, remaining: null } : s));
}

// Refresh the "cards left" counts for the drain phase. Cram has a fixed length,
// so it needs none. Non-fatal — a failure just leaves the last known counts.
async function refreshCounts() {
    const s = get(session);
    if (!s || s.config.study === 'cram' || s.extended) return;
    const includeReviews = s.config.study !== 'new';
    const newRemaining =
        s.config.study === 'review' ? 0 : Math.max(0, s.config.length - s.newServed);
    try {
        const rc = await getQueueCounts(s.config.pack || undefined, newRemaining, includeReviews);
        session.update((cur) => (cur ? { ...cur, remaining: rc } : cur));
    } catch {
        /* leave prior counts */
    }
}

// Fetch the next card from the SRS queue and stash it as `current`. Returns null
// when the queue is drained (new budget spent + nothing due) — session is over.
export async function loadQuestion(): Promise<QuestionDto | null> {
    const s = get(session);
    if (!s || s.finished) return null;

    // Cram is a fixed-length practice run (ignores the schedule); it ends after
    // `length` questions rather than draining a queue.
    if (s.config.study === 'cram') {
        if (s.answers.length >= s.config.length) {
            session.update((cur) => (cur ? { ...cur, current: null, finished: true } : cur));
            return null;
        }
        const cq = await getNextQuestion(s.config.pack || undefined, 0, false, true);
        if (!cq) {
            session.update((cur) => (cur ? { ...cur, current: null, finished: true } : cur));
            return null;
        }
        const shuffledCram: QuestionDto = { ...cq, choices: shuffle(cq.choices) };
        session.update((cur) => (cur ? { ...cur, current: shuffledCram } : cur));
        return shuffledCram;
    }

    // 'review' spends no new-card budget; 'new' skips the review queue.
    const includeReviews = s.config.study !== 'new';
    const newRemaining =
        s.config.study === 'review' ? 0 : Math.max(0, s.config.length - s.newServed);
    const q = await getNextQuestion(s.config.pack || undefined, newRemaining, includeReviews);
    if (!q) {
        // Scheduled queue drained (new budget spent + nothing due/learning).
        if (s.extended) {
            // Study-ahead: pull any card, soonest-due first (cram-style). Only
            // truly finished when even that is empty (no birds in the pool).
            const cq = await getNextQuestion(s.config.pack || undefined, 0, false, true);
            if (!cq) {
                session.update((cur) => (cur ? { ...cur, current: null, finished: true } : cur));
                return null;
            }
            const sc: QuestionDto = { ...cq, choices: shuffle(cq.choices) };
            session.update((cur) => (cur ? { ...cur, current: sc } : cur));
            return sc;
        }
        // Not finished — the UI shows a "scheduled cards done" screen offering
        // summary-or-continue. `finished` stays false so this isn't the results exit.
        session.update((cur) => (cur ? { ...cur, current: null } : cur));
        return null;
    }
    const shuffled: QuestionDto = { ...q, choices: shuffle(q.choices) };
    session.update((cur) => (cur ? { ...cur, current: shuffled } : cur));
    void refreshCounts();
    return shuffled;
}

// Submit the guess for the current question, record the result, advance.
// `skipped` marks a deliberate "I don't know" (still counts as not-correct).
export async function answerCurrent(guess: string, skipped = false): Promise<AnswerRecord | null> {
    const s = get(session);
    if (!s || !s.current) return null;

    const result = await submitAnswer(s.current.bird_id, s.current.recording_id, guess);
    const record: AnswerRecord = {
        bird_id: s.current.bird_id,
        recording_id: s.current.recording_id,
        guess,
        correct: result.correct,
        correct_name: result.correct_name,
        skipped
    };

    session.update((cur) => {
        if (!cur) return cur;
        // A new bird just answered spends one of the new-card budget. `finished`
        // is decided by loadQuestion (queue drain), not a fixed question count.
        const newServed = cur.newServed + (cur.current?.is_new ? 1 : 0);
        return {
            ...cur,
            answers: [...cur.answers, record],
            current: null,
            index: cur.index + 1,
            newServed
        };
    });

    // Update the "cards left" counts to reflect this answer's rescheduling.
    void refreshCounts();

    return record;
}

export function score(s: SessionState): number {
    return s.answers.filter((a) => a.correct).length;
}

export function endSession() {
    session.set(null);
}
