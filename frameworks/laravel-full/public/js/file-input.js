document.addEventListener("DOMContentLoaded", () => {
    const fileInput = document.getElementById("File-Input");
    const fileLabel = document.getElementById("File-Label");

    fileInput.addEventListener("change", () => {
        if (fileInput.files.length > 0) {
            // Get the selected file's name
            const fileName = fileInput.files[0].name;

            // Update the label text
            fileLabel.textContent = fileName;

            // Add the font-semibold class
            fileLabel.classList.add("font-semibold");
            fileLabel.classList.remove("text-tedja-secondary");
        } else {
            // Reset the label text and remove the font-semibold class if no file is selected
            fileLabel.textContent = "Add an attachment to request";
            fileLabel.classList.add("text-tedja-secondary");
            fileLabel.classList.remove("font-semibold");
        }
    });
});