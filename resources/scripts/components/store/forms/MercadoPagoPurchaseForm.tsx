import tw from 'twin.macro';
import { Form, Formik } from 'formik';
import React, { useState } from 'react';
import useFlash from '@/plugins/useFlash';
import mpago from '@/api/store/gateways/mercadopago';
import Select from '@/components/elements/Select';
import { Dialog } from '@/components/elements/dialog';
import { Button } from '@/components/elements/button/index';
import TitledGreyBox from '@/components/elements/TitledGreyBox';
import FlashMessageRender from '@/components/FlashMessageRender';
import SpinnerOverlay from '@/components/elements/SpinnerOverlay';

export default () => {
    const { clearAndAddHttpError } = useFlash();
    const [amount, setAmount] = useState(0);
    const [submitting, setSubmitting] = useState(false);

    const submit = () => {
        setSubmitting(true);

        mpago(amount)
            .then((url) => {
                setSubmitting(false);

                // @ts-expect-error this is valid
                window.location.href = url;
            })
            .catch((error) => {
                setSubmitting(false);

                clearAndAddHttpError({ key: 'store:mpago', error });
            });
    };

    return (
        <TitledGreyBox title={'Comprar via Mercado Pago'}>
            <Dialog open={submitting} hideCloseIcon onClose={() => undefined}>
                Agora você está sendo levado ao gateway Mercado Pago para concluir essa transação.
            </Dialog>
            <FlashMessageRender byKey={'store:mpago'} css={tw`mb-2`} />
            <Formik
                onSubmit={submit}
                initialValues={{
                    amount: 100,
                }}
            >
                <Form>
                    <SpinnerOverlay size={'large'} visible={submitting} />
                    <Select name={'amount'} disabled={submitting} onChange={(e) => setAmount(parseInt(e.target.value))}>
                        <option key={'mpago:buy:0'} value={0}>
                            Escolha um valor...
                        </option>
                        <option key={'mpago:buy:100'} value={100}>
                            Comprar 100 creditos
                        </option>
                        <option key={'mpago:buy:200'} value={200}>
                            Comprar 200 creditos
                        </option>
                        <option key={'mpago:buy:500'} value={500}>
                            Comprar 500 creditos
                        </option>
                        <option key={'mpago:buy:1000'} value={1000}>
                            Comprar 1000 creditos
                        </option>
                        <option key={'mpago:buy:2000'} value={2000}>
                            Comprar 2000 creditos
                        </option>
                        <option key={'mpago:buy:4000'} value={4000}>
                            Comprar 4000 creditos
                        </option>
                        <option key={'mpago:buy:6000'} value={6000}>
                            Comprar 6000 creditos
                        </option>
                        <option key={'mpago:buy:8000'} value={8000}>
                            Comprar 8000 creditos
                        </option>
                        <option key={'mpago:buy:10000'} value={10000}>
                            Comprar 10000 creditos
                        </option>
                    </Select>
                    <div css={tw`mt-6`}>
                        <Button type={'submit'} disabled={submitting}>
                            Comprar via Mercado Pago
                        </Button>
                    </div>
                </Form>
            </Formik>
        </TitledGreyBox>
    );
};
