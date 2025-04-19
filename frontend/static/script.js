export function generateLinks() {
    let date_val = document.querySelector('input[type="date"]').valueAsNumber;
    const endpoint = `http://127.0.0.1:3001/video_data?timestamp=${date_val}`;
    const linksContainer = document.getElementById("links-container");
    linksContainer.replaceChildren();
    fetch(endpoint)
        .then(response => response.json())
        .then(data => {
            console.info(data);
            data.forEach(item => {
                console.info("Item to make link:", item);
                const details_section = document.createElement("details");
                let created = item.video_created;
                let duration = item.video_duration;
                let file_name = item.file_name;
                const summary = document.createElement("summary");
                summary.textContent = `Video datetime: ${created}, Video length: ${duration}`;
                details_section.appendChild(summary);
                details_section.style.display = "block";
                const video_section = document.createElement("video");
                video_section.controls = true;
                const source = document.createElement("source");
                source.src = `http://127.0.0.0:3001/file?filename=${file_name}`;
                source.type = "video/mp4";
                video_section.appendChild(source);
                details_section.appendChild(video_section);
                linksContainer.appendChild(details_section);
            });
        }
        )
        .catch(error => {
            console.error("Error fetching data:", error);
        })
};


export function get_cam_status() {
    const endpoint = "http://127.0.0.0:3001/cam_status";
    const status_text = document.getElementById("current_status");
    fetch(endpoint)
        .then(response => response.json())
        .then(data => {
            console.log(data);
            status_text.textContent = data.message;
        })
}


export async function start_recording_mode() {
    const endpoint = "http://127.0.0.0:3001/start_cam?recording_type=Record";
    const status_text = document.getElementById("current_status");
    const response = await fetch(endpoint)
    if (response.ok()) {
        status_text.textContent = "Record";
    } else {
        console.error("Error occured trying to start camera status:", response.status);
    }
}


export async function start_streaming_mode() {
    const endpoint = "http://127.0.0.0:3001/start_cam?recording_type=Stream";
    const status_text = document.getElementById("current_status");
    const streaming_section = document.getElementById("streaming_section");
    const response = await fetch(endpoint)
    if (response.ok()) {
        status_text.textContent = "Stream";
        let link = document.createElement("a");
        link.href = "http://127.0.0.0:3001/watch_stream";
        link.text = "Watch Stream"
        streaming_section.appendChild
    } else {
        console.error("Error occured trying to start camera status:", response.status);
    }
}
