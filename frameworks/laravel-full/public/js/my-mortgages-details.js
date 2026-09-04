// Ambil elemen-elemen tab dan konten
const tabButtons = document.querySelectorAll("#TabButtons button");
const tabContents = document.querySelectorAll(".TabValues > section");

// Fungsi untuk mengaktifkan tab
function activateTab(index) {
    // Nonaktifkan semua tab
    tabButtons.forEach((button, btnIndex) => {
        const title = button.querySelector("h3");

        if (btnIndex === index) {
            title.classList.add("text-[#FAFAFA]");
            title.classList.remove("text-tedja-black");
            button.classList.add("bg-tedja-black");
            button.classList.remove("bg-white","border-tedja-black");
            button.classList.add("border-transparent");
            button.classList.add("border");
        } else {
            title.classList.remove("text-[#FAFAFA]");
            title.classList.add("text-tedja-black");
            button.classList.remove("bg-tedja-black");
            button.classList.add("bg-white", "border-tedja-black");
            button.classList.remove("border-transparent");
        }
    });

    // Sembunyikan semua konten dan tampilkan hanya yang aktif
    tabContents.forEach((content, contentIndex) => {
        content.style.display = contentIndex === index ? "block" : "none";
    });
}

// Tambahkan event listener ke setiap tombol tab
tabButtons.forEach((button, index) => {
    button.addEventListener("click", () => {
        activateTab(index);
    });
});

// Aktifkan tab pertama saat pertama kali dimuat
activateTab(0);
