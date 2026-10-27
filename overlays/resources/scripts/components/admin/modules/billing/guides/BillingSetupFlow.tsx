import { useState } from 'react';
import { useStoreState } from '@/state/hooks';
import ChooseGatewayDialog, { BillingGateway } from './ChooseGatewayDialog';
import SetupStripe from './SetupStripe';
import SetupMercadoPago from './SetupMercadoPago';

// Fluxo de configuração inicial do faturamento (modo automático): enquanto
// nenhum provedor estiver configurado, exibe o diálogo neutro de escolha;
// depois abre o guia do provedor selecionado. Dispensável com o X.
export default () => {
    const settings = useStoreState(s => s.everest.data!.billing);
    const [selected, setSelected] = useState<BillingGateway | null>(null);
    const [dismissed, setDismissed] = useState(false);

    const stripeOk = Boolean(settings.keys.secret);
    const mercadoPagoOk = Boolean(settings.mercadopago?.access_token);

    if (stripeOk || mercadoPagoOk || dismissed) return null;

    if (selected === 'stripe') {
        return <SetupStripe onClose={() => setDismissed(true)} />;
    }

    if (selected === 'mercadopago') {
        return <SetupMercadoPago onClose={() => setDismissed(true)} />;
    }

    return <ChooseGatewayDialog onClose={() => setDismissed(true)} onSelect={setSelected} />;
};
