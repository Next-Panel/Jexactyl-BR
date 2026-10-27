import Input from '@/elements/Input';
import { useStoreState } from '@/state/hooks';
import { Dialog } from '@/elements/dialog';
import { faExclamationTriangle, faCheckCircle, faInfoCircle } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import Tooltip from '@/elements/tooltip/Tooltip';
import { useState } from 'react';
import { Button } from '@/elements/button';
import { updateSettings } from '@/api/routes/admin/billing';

interface MercadoPagoKeys {
    access_token?: string;
    webhook_secret?: string;
}

export default ({ onClose }: { onClose?: () => void }) => {
    const [data, setData] = useState<MercadoPagoKeys>();
    const [dismissed, setDismissed] = useState(false);
    const existingKeys = useStoreState(s => s.everest.data!.billing.mercadopago);

    const submit = async () => {
        if (!data || !data.access_token) return;

        await updateSettings('mercadopago:access_token', data.access_token);
        if (data.webhook_secret) {
            await updateSettings('mercadopago:webhook_secret', data.webhook_secret);
        }
        window.location.reload();
    };

    // Sem onClose, abre sozinho quando nao ha token, como antes.
    const open = onClose ? true : !dismissed && !!existingKeys && !existingKeys.access_token;
    const handleClose = onClose ?? (() => setDismissed(true));

    return (
        <Dialog open={open} onClose={handleClose} title={'Configurar Mercado Pago'}>
            <div className={'p-3 bg-black/50 rounded-lg mb-4'}>
                <p className={'text-gray-200 font-semibold'}>
                    <FontAwesomeIcon icon={faInfoCircle} className={'text-blue-400 mr-2'} />
                    Ainda configurando?
                </p>
                <p className={'text-gray-400 text-sm'}>
                    Fique à vontade para ignorar esta mensagem fechando o diálogo e continuar configurando seus produtos
                    e categorias. Quando estiver pronto, vá para a aba Configurações para informar o token de acesso.
                </p>
            </div>
            Antes de usar o Mercado Pago, você deve fornecer ao Jexpanel um token de acesso para autenticar com o
            Mercado Pago. Visite o painel de credenciais do Mercado Pago
            <a
                target={'_blank'}
                rel={'noreferrer'}
                className={'text-blue-300 mx-1'}
                href={'https://www.mercadopago.com.br/developers/panel/credentials'}
            >
                aqui
            </a>
            para obter o token de acesso e o segredo do webhook, depois cole-os aqui.
            <div className={'relative mt-4'}>
                <Input
                    placeholder={'Informe o token de acesso aqui...'}
                    onChange={e => setData({ ...data, access_token: e.currentTarget.value })}
                />
                {!data?.access_token ? (
                    <Tooltip
                        placement={'right'}
                        content={'Você deve informar um token de acesso válido do Mercado Pago para continuar.'}
                    >
                        <FontAwesomeIcon
                            icon={faExclamationTriangle}
                            className={'absolute top-1/3 right-4 text-yellow-500'}
                        />
                    </Tooltip>
                ) : (
                    <FontAwesomeIcon icon={faCheckCircle} className={'absolute top-1/3 right-4 text-green-500'} />
                )}
            </div>
            <div className={'relative mt-4'}>
                <Input
                    placeholder={'Informe o segredo do webhook (opcional)...'}
                    onChange={e => setData({ ...data, webhook_secret: e.currentTarget.value })}
                />
            </div>
            <div className={'w-full text-right mt-4'}>
                <Button onClick={submit} disabled={!data?.access_token}>
                    Enviar
                </Button>
            </div>
        </Dialog>
    );
};
