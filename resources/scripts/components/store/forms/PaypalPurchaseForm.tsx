import tw from 'twin.macro';
import { Form, Formik } from 'formik';
import React, { useState } from 'react';
import useFlash from '@/plugins/useFlash';
import paypal from '@/api/store/gateways/paypal';
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

        paypal(amount)
            .then((url) => {
                setSubmitting(false);

                // @ts-expect-error this is valid
                window.location.href = url;
            })
            .catch((error) => {
                setSubmitting(false);

                clearAndAddHttpError({ key: 'store:paypal', error });
            });
    };

    return (
        <TitledGreyBox title={'Comprar via PayPal'}>
            <Dialog open={submitting} hideCloseIcon onClose={() => undefined}>
                Agora você está sendo levado ao gateway PayPal para concluir essa transação.
            </Dialog>
            <FlashMessageRender byKey={'store:paypal'} css={tw`mb-2`} />
            <Formik
                onSubmit={submit}
                initialValues={{
                    amount: 100,
                }}
            >
                <Form>
                    <SpinnerOverlay size={'large'} visible={submitting} />
                    <Select
                        name={'amount'}
                        disabled={submitting}
                        // @ts-expect-error this is valid
                        onChange={(e) => setAmount(e.target.value)}
                    >
                        <option key={'paypal:placeholder'} hidden>
                            Escolha um valor...
                        </option>
                        <option key={'paypal:buy:100'} value={100}>
                            Comprar 100 creditos
                        </option>
                        <option key={'paypal:buy:200'} value={200}>
                            Comprar 200 creditos
                        </option>
                        <option key={'paypal:buy:500'} value={500}>
                            Comprar 500 creditos
                        </option>
                        <option key={'paypal:buy:1000'} value={1000}>
                            Comprar 1000 creditos
                        </option>
                    </Select>
                    <div css={tw`mt-6`}>
                        <Button type={'submit'} disabled={submitting}>
                            Comprar via PayPal
                        </Button>
                    </div>
                </Form>
            </Formik>
        </TitledGreyBox>
    );
};
