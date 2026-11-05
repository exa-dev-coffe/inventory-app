use rust_decimal::Decimal;
use uuid::Uuid;

use inventory_app::modules::opname::model::{
    CreateOpnameDto, PhysicalCountItemDto, SubmitOpnameCountDto,
};

#[test]
fn test_create_opname_dto() {
    let ing1 = Uuid::new_v4();
    let ing2 = Uuid::new_v4();

    let dto = CreateOpnameDto {
        notes: Some("Closing Shift Opname".to_string()),
        ingredient_ids: Some(vec![ing1, ing2]),
    };

    assert_eq!(dto.notes.unwrap(), "Closing Shift Opname");
    assert_eq!(dto.ingredient_ids.unwrap().len(), 2);
}

#[test]
fn test_submit_opname_counts_dto() {
    let ing = Uuid::new_v4();
    let count_item = PhysicalCountItemDto {
        ingredient_id: ing,
        physical_stock: Decimal::new(485, 1), // 48.5
        notes: Some("Slightly spilled during transfer".to_string()),
    };

    let dto = SubmitOpnameCountDto {
        items: vec![count_item],
    };

    assert_eq!(dto.items.len(), 1);
    assert_eq!(dto.items[0].physical_stock, Decimal::new(485, 1));
}
