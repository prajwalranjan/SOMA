import { NoteInput } from "./components/NoteInput";
import { useNotes } from "./hooks/useNotes";
import "./App.css";

function App() {
  const { notes, loading, createNote } = useNotes();

  return (
    <main style={{ maxWidth: "800px", margin: "0 auto", padding: "1rem" }}>
      <h1>SOMA</h1>
      <p style={{ color: "#888", marginTop: "-0.5rem" }}>
        Semantic Offline Memory Assistant
      </p>

      <NoteInput createNote={createNote} />

      <hr />

      <h2>Notes</h2>
      {loading && <p>Loading...</p>}
      {notes.length === 0 && !loading && (
        <p style={{ color: "#888" }}>No notes yet. Start capturing thoughts.</p>
      )}
      {notes.map((note) => (
        <div
          key={note.id}
          style={{
            border: "1px solid #333",
            borderRadius: "8px",
            padding: "1rem",
            marginBottom: "0.75rem",
          }}
        >
          <p style={{ margin: 0 }}>{note.content}</p>
          <small style={{ color: "#888" }}>
            {new Date(note.logged_at).toLocaleString()}
            {note.thought_at !== note.logged_at && (
              <> · thought at {new Date(note.thought_at).toLocaleString()}</>
            )}
          </small>
        </div>
      ))}
    </main>
  );
}

export default App;