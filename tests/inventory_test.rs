use rust_decimal::Decimal;
use uuid::Uuid;

use inventory_app::modules::ingredient::model::CreateIngredientDto;
use inventory_app::modules::recipe::model::{RecipeIngredientItemDto, SetRecipeDto};
use inventory_app::modules::stock::model::StockAdjustmentDto;

#[test]
fn test_create_ingredient_dto() {
    let dto = CreateIngredientDto {
        sku: "BEAN-GAYO-01".to_string(),
        name: "Arabica Gayo Beans".to_string(),
        category: "beans".to_string(),
        unit: "gram".to_string(),
        initial_stock: Some(Decimal::from(5000)),
        min_stock_alert: Some(Decimal::from(500)),
        cost_per_unit: Some(Decimal::from(250)),
    };

    assert_eq!(dto.sku, "BEAN-GAYO-01");
    assert_eq!(dto.unit, "gram");
    assert_eq!(dto.initial_stock, Some(Decimal::from(5000)));
}

#[test]
fn test_set_recipe_dto() {
    let ing_id1 = Uuid::new_v4();
    let ing_id2 = Uuid::new_v4();

    let dto = SetRecipeDto {
        menu_id: 101, // e.g. Cappuccino
        ingredients: vec![
            RecipeIngredientItemDto {
                ingredient_id: ing_id1,
                amount: Decimal::from(18), // 18g beans
            },
            RecipeIngredientItemDto {
                ingredient_id: ing_id2,
                amount: Decimal::from(150), // 150ml milk
            },
        ],
    };

    assert_eq!(dto.menu_id, 101);
    assert_eq!(dto.ingredients.len(), 2);
    assert_eq!(dto.ingredients[0].amount, Decimal::from(18));
}

#[test]
fn test_stock_adjustment_dto() {
    let ing_id = Uuid::new_v4();
    let adj = StockAdjustmentDto {
        ingredient_id: ing_id,
        change_type: "IN_PURCHASE".to_string(),
        quantity: Decimal::from(10000), // +10kg
        reference_id: Some("PO-2026-001".to_string()),
        notes: Some("Fresh roast delivery".to_string()),
    };

    assert_eq!(adj.change_type, "IN_PURCHASE");
    assert_eq!(adj.quantity, Decimal::from(10000));
}

