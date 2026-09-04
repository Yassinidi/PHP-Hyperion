document.addEventListener("DOMContentLoaded", () => {
    const profileButton = document.getElementById("Profile");
    const dropdownMenu = profileButton.querySelector("ul");

    // Toggle dropdown on button click
    profileButton.addEventListener("click", (e) => {
        e.stopPropagation(); // Prevent event from bubbling to document
        dropdownMenu.classList.toggle("hidden");
    });

    // Close dropdown when clicking outside
    document.addEventListener("click", () => {
        if (!dropdownMenu.classList.contains("hidden")) {
            dropdownMenu.classList.add("hidden");
        }
    });

    // Close dropdown when clicking an item in the dropdown
    dropdownMenu.addEventListener("click", (e) => {
        e.stopPropagation(); // Prevent event from bubbling to document
        dropdownMenu.classList.add("hidden");
    });
});
