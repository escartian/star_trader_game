pub fn ui(ui: &mut egui::Ui) {
    ui.separator();
    ui.heading("Market layout fixture");
    ui.label("Sample prices only; trading is not implemented.");
    egui::ScrollArea::both()
        .id_salt("market")
        .max_height(150.0)
        .show(ui, |ui| {
            egui::Grid::new("market_table")
                .striped(true)
                .min_col_width(75.0)
                .show(ui, |ui| {
                    for title in ["Commodity", "Buy (cr)", "Sell (cr)", "Stock"] {
                        ui.strong(title);
                    }
                    ui.end_row();
                    for (name, buy, sell, stock) in [
                        ("Food", 12, 10, 80),
                        ("Ore", 30, 26, 40),
                        ("Medicine", 70, 62, 15),
                        ("Machinery", 90, 80, 20),
                        ("Textiles", 20, 17, 65),
                        ("Electronics", 110, 98, 10),
                    ] {
                        ui.label(name);
                        ui.label(buy.to_string());
                        ui.label(sell.to_string());
                        ui.label(stock.to_string());
                        ui.end_row();
                    }
                });
        });
}
