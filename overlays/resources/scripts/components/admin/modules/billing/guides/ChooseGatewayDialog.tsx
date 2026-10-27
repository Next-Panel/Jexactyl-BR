import { Dialog } from '@/elements/dialog';
import { Button } from '@/elements/button';

export type BillingGateway = 'stripe' | 'mercadopago';

interface Props {
    onClose: () => void;
    onSelect: (gateway: BillingGateway) => void;
}

// Diálogo neutro para escolher qual provedor configurar. Exibido sempre
// que nenhum gateway estiver configurado, em vez de abrir direto o
// formulário de um provedor específico.
export default ({ onClose, onSelect }: Props) => {
    return (
        <Dialog open onClose={onClose} title={'Configurar pagamentos'}>
            <p className={'text-gray-400 text-sm mb-4'}>
                Escolha qual provedor você quer configurar primeiro. É possível ativar os dois e definir o padrão
                depois na tela de faturamento.
            </p>
            <div className={'flex flex-col gap-3'}>
                <button
                    onClick={() => onSelect('stripe')}
                    className={
                        'text-left p-3 rounded-lg bg-black/50 hover:bg-black/70 transition-colors border border-transparent hover:border-blue-400'
                    }
                >
                    <p className={'text-gray-200 font-semibold'}>Stripe</p>
                    <p className={'text-gray-400 text-sm'}>Cartões de crédito e débito internacionais.</p>
                </button>
                <button
                    onClick={() => onSelect('mercadopago')}
                    className={
                        'text-left p-3 rounded-lg bg-black/50 hover:bg-black/70 transition-colors border border-transparent hover:border-blue-400'
                    }
                >
                    <p className={'text-gray-200 font-semibold'}>Mercado Pago</p>
                    <p className={'text-gray-400 text-sm'}>Pix, cartão e boleto.</p>
                </button>
            </div>
            <div className={'w-full text-right mt-4'}>
                <Button.Text onClick={onClose}>Agora não</Button.Text>
            </div>
        </Dialog>
    );
};
