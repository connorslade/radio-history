fetch("/messages")
  .then((r) => r.json())
  .then((messages) => {
    let tbody = document.querySelector("#messages");
    for (let message of messages) {
      let tr = document.createElement("tr");
      tr.innerHTML = `
            <td>${message.date}</td>
            <td>${message.text}</td>
            <td class="center"><a href="/audio/${message.audio}">▶</a></td>
        `;
      tbody.appendChild(tr);
    }
  });
