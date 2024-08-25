function add_message(message, top) {
  let tr = document.createElement("tr");
  tr.innerHTML = `
            <td>${message.date}</td>
            <td>${message.text}</td>
            <td class="center"><a href="/audio/${message.audio}">▶</a></td>
        `;

  if (top) tbody.insertBefore(tr, tbody.firstChild);
  else tbody.appendChild(tr);
}

function set_processing(processing) {
  let processing_message = document.querySelector("#processing");
  processing_message.style.display = processing != null ? "block" : "none";
  processing_message.innerHTML = processing;
}

let tbody = document.querySelector("#messages");

fetch("/messages")
  .then((r) => r.json())
  .then((messages) => {
    for (let message of messages) add_message(message, false);
  });

let ws = new WebSocket(`ws://${location.host}/events`);
ws.onmessage = (event) => {
  let message = JSON.parse(event.data);
  if (message.type === "Receiving") set_processing("Receiving...");
  else if (message.type === "Processing") set_processing("Processing...");
  else if (message.type === "Complete") {
    set_processing(null);
    add_message(message, true);
  }
};
