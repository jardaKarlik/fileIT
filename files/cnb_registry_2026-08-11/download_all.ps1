$baseUrl = "https://jerrs.cnb.cz/apljerrsdad/JERRS.WEB35.STD_SES_XML_EXP2"
$date = "11.08.2026"
$outDir = "c:\_temp\xml"

$categories = @(
    # Banky a družstevní záložny
    @{ idx = 1;   file = "01_banks_foreign_branches.xml" },
    @{ idx = 16;  file = "02_cooperative_banks.xml" },
    @{ idx = 133; file = "03_foreign_bank_representations.xml" },
    @{ idx = 134; file = "04_foreign_financial_institutions.xml" },
    @{ idx = 412; file = "05_financial_holding_companies.xml" },
    # Obchodníci s CP
    @{ idx = 37;  file = "06_securities_dealers_branches.xml" },
    @{ idx = 49;  file = "07_systematic_internalizers.xml" },
    @{ idx = 478; file = "08_designated_publishers.xml" },
    @{ idx = 51;  file = "09_independent_record_keepers.xml" },
    @{ idx = 140; file = "10_foreign_securities_dealers.xml" },
    @{ idx = 115; file = "11_investment_brokers.xml" },
    @{ idx = 127; file = "12_bound_reps_capital_market.xml" },
    @{ idx = 132; file = "13_bound_reps_for_principal_capital.xml" },
    @{ idx = 351; file = "14_bound_reps_foreign_securities.xml" },
    @{ idx = 350; file = "15_accredited_persons_capital_market.xml" },
    # Pojišťovny a zajišťovny
    @{ idx = 17;  file = "16_insurers_branches.xml" },
    @{ idx = 31;  file = "17_reinsurers.xml" },
    @{ idx = 139; file = "18_foreign_insurers.xml" },
    @{ idx = 376; file = "19_insurance_brokers.xml" },
    @{ idx = 377; file = "20_bound_reps_insurance.xml" },
    @{ idx = 383; file = "21_bound_reps_for_principal_insurance.xml" },
    @{ idx = 378; file = "22_supplementary_brokers.xml" },
    @{ idx = 379; file = "23_supplementary_brokers_for_principal.xml" },
    @{ idx = 186; file = "24_foreign_insurance_brokers.xml" },
    @{ idx = 502; file = "25_insurance_broker_policyholders.xml" },
    @{ idx = 507; file = "26_broker_policyholders_linked.xml" },
    @{ idx = 380; file = "27_accredited_persons_insurance.xml" },
    @{ idx = 501; file = "28_other_insurance_entities.xml" },
    # Panevropské osobní penzijní produkty
    @{ idx = 399; file = "29_pepp_products.xml" },
    @{ idx = 400; file = "30_pepp_providers_distributors.xml" },
    @{ idx = 396; file = "31_pepp_providers.xml" },
    @{ idx = 398; file = "32_pepp_foreign_providers.xml" },
    @{ idx = 397; file = "33_pepp_distributors.xml" },
    # Poskytovatelé DIP
    @{ idx = 431; file = "34_dip_providers.xml" },
    @{ idx = 430; file = "35_dip_foreign_providers.xml" },
    # Investiční společnosti a fondy
    @{ idx = 53;  file = "36_investment_companies_branches.xml" },
    @{ idx = 204; file = "37_investment_companies_portfolio.xml" },
    @{ idx = 310; file = "38_eusef_funds.xml" },
    @{ idx = 311; file = "39_euveca_funds.xml" },
    @{ idx = 236; file = "40_main_administrators.xml" },
    @{ idx = 354; file = "41_fund_administrators.xml" },
    @{ idx = 239; file = "42_fund_depositories.xml" },
    @{ idx = 248; file = "43_investment_funds_legal.xml" },
    @{ idx = 249; file = "44_self_administered_funds.xml" },
    @{ idx = 250; file = "45_non_self_administered_funds.xml" },
    @{ idx = 251; file = "46_legal_funds_and_depository.xml" },
    @{ idx = 539; file = "47_sub_funds.xml" },
    @{ idx = 247; file = "48_trust_funds.xml" },
    @{ idx = 252; file = "49_trust_funds_and_depository.xml" },
    @{ idx = 82;  file = "50_mutual_funds.xml" },
    @{ idx = 86;  file = "51_mutual_funds_and_depository.xml" },
    @{ idx = 375; file = "52_funds_with_managers_admin.xml" },
    @{ idx = 141; file = "53_foreign_managers_ucits.xml" },
    @{ idx = 235; file = "54_foreign_managers_aifmd.xml" },
    @{ idx = 237; file = "55_foreign_fund_administrators.xml" },
    @{ idx = 253; file = "56_foreign_investment_funds.xml" },
    @{ idx = 254; file = "57_foreign_standard_funds.xml" },
    @{ idx = 256; file = "58_foreign_special_funds.xml" },
    @{ idx = 314; file = "59_foreign_qualified_funds_limit.xml" },
    @{ idx = 315; file = "60_foreign_qualified_funds_nolimit.xml" },
    @{ idx = 308; file = "61_foreign_eusef_funds.xml" },
    @{ idx = 309; file = "62_foreign_euveca_funds.xml" },
    @{ idx = 302; file = "63_liquidators_investment.xml" },
    @{ idx = 303; file = "64_forced_managers_investment.xml" },
    # Penzijní společnosti a fondy
    @{ idx = 223; file = "65_pension_companies.xml" },
    @{ idx = 226; file = "66_participant_funds.xml" },
    @{ idx = 227; file = "67_participant_funds_depository.xml" },
    @{ idx = 224; file = "68_transformed_funds.xml" },
    @{ idx = 225; file = "69_transformed_funds_depository.xml" },
    @{ idx = 382; file = "70_depositories_pension_transformed.xml" },
    @{ idx = 231; file = "71_foreign_pension_institutions.xml" },
    @{ idx = 389; file = "72_brokers_supplementary_pension.xml" },
    @{ idx = 390; file = "73_bound_reps_pension.xml" },
    @{ idx = 391; file = "74_bound_reps_for_principal_pension.xml" },
    @{ idx = 222; file = "75_accredited_persons_pension.xml" },
    # Platební instituce
    @{ idx = 174; file = "76_payment_institutions.xml" },
    @{ idx = 184; file = "77_small_payment_providers.xml" },
    @{ idx = 355; file = "78_account_info_managers.xml" },
    @{ idx = 182; file = "79_electronic_money_institutions.xml" },
    @{ idx = 183; file = "80_small_electronic_money_issuers.xml" },
    @{ idx = 180; file = "81_authorized_reps_payment.xml" },
    @{ idx = 181; file = "82_authorized_reps_for_principal_payment.xml" },
    @{ idx = 205; file = "83_foreign_payment_institutions.xml" },
    @{ idx = 356; file = "84_foreign_account_info_managers.xml" },
    @{ idx = 206; file = "85_foreign_electronic_money.xml" },
    @{ idx = 212; file = "86_authorized_reps_foreign_payment.xml" },
    @{ idx = 359; file = "87_insurers_payment_services.xml" },
    @{ idx = 384; file = "88_third_party_providers.xml" },
    @{ idx = 428; file = "89_dynamic_currency_exchange.xml" },
    # Trhy s kryptoaktivy
    @{ idx = 457; file = "90_crypto_white_papers.xml" },
    @{ idx = 458; file = "91_token_issuers_authorized.xml" },
    @{ idx = 459; file = "92_token_issuers_notified_mica17.xml" },
    @{ idx = 460; file = "93_foreign_token_issuers.xml" },
    @{ idx = 461; file = "94_e_money_token_issuers_mica48.xml" },
    @{ idx = 466; file = "95_foreign_e_money_token_issuers.xml" },
    @{ idx = 467; file = "96_crypto_service_providers_auth_mica62.xml" },
    @{ idx = 468; file = "97_crypto_service_providers_notified_mica60.xml" },
    @{ idx = 476; file = "98_foreign_crypto_service_providers.xml" },
    # Spotřebitelský úvěr
    @{ idx = 339; file = "99_nonbank_consumer_lenders.xml" },
    @{ idx = 340; file = "100_consumer_credit_brokers.xml" },
    @{ idx = 341; file = "101_bound_reps_consumer_credit.xml" },
    @{ idx = 345; file = "102_bound_reps_for_principal_credit.xml" },
    @{ idx = 342; file = "103_tied_consumer_credit_brokers.xml" },
    @{ idx = 346; file = "104_tied_brokers_for_principal_credit.xml" },
    @{ idx = 343; file = "105_foreign_housing_credit_brokers.xml" },
    @{ idx = 344; file = "106_accredited_persons_consumer_credit.xml" },
    # Správci nevýkonných úvěrů
    @{ idx = 448; file = "107_npl_managers.xml" },
    @{ idx = 450; file = "108_foreign_npl_managers.xml" },
    @{ idx = 451; file = "109_other_npl_managers.xml" },
    # Směnárny
    @{ idx = 118; file = "110_exchange_dealers.xml" },
    @{ idx = 124; file = "111_exchange_points.xml" },
    @{ idx = 304; file = "112_other_exchange_points.xml" },
    # Skupinové financování
    @{ idx = 411; file = "113_crowdfunding_providers.xml" },
    @{ idx = 429; file = "114_foreign_crowdfunding_providers.xml" },
    # Ostatní
    @{ idx = 112; file = "115_central_depository.xml" },
    @{ idx = 113; file = "116_central_depository_participants.xml" },
    @{ idx = 146; file = "117_issuers_regulated_market.xml" },
    @{ idx = 52;  file = "118_regulated_market_organizer.xml" },
    @{ idx = 348; file = "119_reporting_data_providers.xml" },
    @{ idx = 352; file = "120_reporting_data_operators.xml" },
    @{ idx = 349; file = "121_sme_growth_markets.xml" },
    @{ idx = 110; file = "122_settlement_systems.xml" },
    @{ idx = 111; file = "123_settlement_participants.xml" },
    @{ idx = 116; file = "124_forced_managers_other.xml" },
    @{ idx = 117; file = "125_liquidators_other.xml" },
    @{ idx = 221; file = "126_banknote_coin_processors.xml" },
    @{ idx = 316; file = "127_payment_systems.xml" },
    @{ idx = 317; file = "128_payment_system_participants.xml" },
    @{ idx = 321; file = "129_resolution_institutions.xml" },
    @{ idx = 353; file = "130_benchmark_administrators.xml" },
    @{ idx = 362; file = "131_entities_in_liquidation.xml" },
    @{ idx = 513; file = "132_license_holders_not_registered.xml" },
    @{ idx = 238; file = "133_zisif_registered_persons.xml" }
)

$total = $categories.Count
$done = 0
$errors = @()

foreach ($cat in $categories) {
    $done++
    $pct = [math]::Round($done / $total * 100)
    Write-Host "[$done/$total] ($pct%) Downloading: $($cat.file) (idx=$($cat.idx))..."
    
    $url = "${baseUrl}?p_lang=cz&p_DATUM=${date}&p_hie=HI&p_ses_idx=$($cat.idx)"
    $outPath = Join-Path $outDir $cat.file
    
    try {
        Invoke-WebRequest -Uri $url -OutFile $outPath -UseBasicParsing -TimeoutSec 30
        $size = (Get-Item $outPath).Length
        Write-Host "  OK ($size bytes)"
    } catch {
        $errors += "$($cat.file): $($_.Exception.Message)"
        Write-Host "  FAILED: $($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host "`n=== DONE ===" -ForegroundColor Green
Write-Host "Downloaded: $($total - $errors.Count) / $total"
if ($errors.Count -gt 0) {
    Write-Host "Errors:" -ForegroundColor Yellow
    $errors | ForEach-Object { Write-Host "  $_" -ForegroundColor Yellow }
}
